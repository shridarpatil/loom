use std::path::{Path, PathBuf};

use clap::Args;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

use loom_core::db::migrate::{migrate_all, migrate_doctype, migrate_system_tables};
use loom_core::doctype::crud;
use loom_core::doctype::meta::Meta;
use loom_core::doctype::DocTypeRegistry;

#[derive(Debug, Args)]
pub struct InstallAppArgs {
    /// App name or path to the app directory
    pub app: String,

    /// Database URL (overrides site config)
    #[arg(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// Site name
    #[arg(long)]
    pub site: Option<String>,
}

/// App metadata from loom_app.toml
#[derive(Debug, Deserialize)]
struct AppConfig {
    app: AppMeta,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AppMeta {
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    modules: Vec<String>,
}

pub async fn run(args: InstallAppArgs) -> anyhow::Result<()> {
    // Resolve app path: check if it's a direct path or under apps/
    let app_path = resolve_app_path(&args.app)?;
    tracing::info!("Installing app from {:?}", app_path);

    // 1. Parse loom_app.toml
    let config_path = app_path.join("loom_app.toml");
    if !config_path.exists() {
        anyhow::bail!(
            "No loom_app.toml found at {:?}. Is this a valid Loom app?",
            config_path
        );
    }
    let config_str = std::fs::read_to_string(&config_path)?;
    let config: AppConfig = toml::from_str(&config_str)?;
    tracing::info!(
        "App: {} v{}",
        config.app.name,
        config.app.version.as_deref().unwrap_or("0.0.0")
    );

    // 2. Connect to database
    let db_url = crate::site_config::resolve_db_url(args.db_url, args.site.as_deref());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    tracing::info!("Connected to database");

    // Ensure system tables exist
    migrate_system_tables(&pool).await?;

    // 3. Set up registry and bootstrap DocType
    let registry = DocTypeRegistry::new();
    let doctype_meta = Meta::doctype_meta();
    migrate_doctype(&pool, &doctype_meta).await?;
    registry.register(doctype_meta).await;

    // Load existing DocTypes from DB
    registry.load_from_database(&pool).await?;

    // 4. Load DocTypes from app directory
    let doctypes_dir = app_path.join("doctypes");
    if doctypes_dir.exists() {
        let count = registry.load_from_directory(&doctypes_dir).await?;
        tracing::info!("Loaded {} DocType(s) from app", count);
    }

    // 5. Run migrations for all DocTypes
    migrate_all(&pool, &registry).await?;
    tracing::info!("Database tables migrated");

    // 6. Sync DocTypes to __doctype table
    registry.sync_to_database(&pool).await?;

    // 7. Load Rhai scripts from doctypes/ directories
    let mut script_count = 0;
    if doctypes_dir.exists() {
        script_count = load_scripts_to_db(&pool, &doctypes_dir).await?;
    }
    if script_count > 0 {
        tracing::info!("Loaded {} script(s)", script_count);
    }

    // 8. Load customizations (.customize.json + .client.js)
    if doctypes_dir.exists() {
        let custom_count = load_customizations(&pool, &doctypes_dir).await?;
        if custom_count > 0 {
            tracing::info!("Loaded {} customization(s)", custom_count);
        }
    }

    // 9. Load fixtures
    let fixtures_dir = app_path.join("fixtures");
    if fixtures_dir.exists() {
        let fixture_count = load_fixtures(&pool, &registry, &fixtures_dir).await?;
        if fixture_count > 0 {
            tracing::info!("Loaded {} fixture record(s)", fixture_count);
        }
    }

    tracing::info!("App '{}' installed successfully!", config.app.name);
    Ok(())
}

/// Resolve the app directory path from a name or path.
fn resolve_app_path(app: &str) -> anyhow::Result<PathBuf> {
    let direct = PathBuf::from(app);
    if direct.is_dir() {
        return Ok(direct);
    }
    let under_apps = PathBuf::from("apps").join(app);
    if under_apps.is_dir() {
        return Ok(under_apps);
    }
    anyhow::bail!(
        "App '{}' not found. Checked '{}' and '{}'",
        app,
        direct.display(),
        under_apps.display()
    )
}

/// Walk doctypes/ for .rhai files and insert them into the __script table.
async fn load_scripts_to_db(pool: &sqlx::PgPool, doctypes_dir: &Path) -> anyhow::Result<usize> {
    let mut count = 0;
    let mut stack = vec![doctypes_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rhai") {
                let script_content = std::fs::read_to_string(&path)?;
                // Derive doctype name from parent directory name
                let doctype_name = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let script_name = path
                    .file_stem()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                sqlx::query(
                    "INSERT INTO \"__script\" (name, doctype, script, modified) \
                     VALUES ($1, $2, $3, NOW()) \
                     ON CONFLICT (name) DO UPDATE SET script = $3, doctype = $2, modified = NOW()",
                )
                .bind(&script_name)
                .bind(&doctype_name)
                .bind(&script_content)
                .execute(pool)
                .await?;

                tracing::info!("Loaded script '{}' for DocType '{}'", script_name, doctype_name);
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Walk doctypes/ for .customize.json and .client.js files and insert into __customization.
async fn load_customizations(pool: &sqlx::PgPool, doctypes_dir: &Path) -> anyhow::Result<usize> {
    let mut count = 0;
    let entries = std::fs::read_dir(doctypes_dir)?;

    for entry in entries.flatten() {
        let dt_dir = entry.path();
        if !dt_dir.is_dir() {
            continue;
        }

        let slug = dt_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Convert slug to DocType name (snake_case → Title Case)
        let doctype_name = slug
            .split('_')
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Load .customize.json (field overrides)
        let customize_path = dt_dir.join(format!("{}.customize.json", slug));
        let overrides: serde_json::Value = if customize_path.exists() {
            let content = std::fs::read_to_string(&customize_path)?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Load .client.js (client script)
        let client_path = dt_dir.join(format!("{}.client.js", slug));
        let client_script = if client_path.exists() {
            std::fs::read_to_string(&client_path)?
        } else {
            String::new()
        };

        let has_overrides = overrides.as_object().is_some_and(|o| !o.is_empty());
        if !has_overrides && client_script.is_empty() {
            continue;
        }

        sqlx::query(
            "INSERT INTO \"__customization\" (doctype, overrides, client_script, modified) \
             VALUES ($1, $2, $3, NOW()) \
             ON CONFLICT (doctype) DO UPDATE SET overrides = $2, client_script = $3, modified = NOW()",
        )
        .bind(&doctype_name)
        .bind(&overrides)
        .bind(&client_script)
        .execute(pool)
        .await?;

        let mut parts = Vec::new();
        if has_overrides { parts.push("field overrides"); }
        if !client_script.is_empty() { parts.push("client script"); }
        tracing::info!("Loaded customization for '{}': {}", doctype_name, parts.join(" + "));
        count += 1;
    }

    Ok(count)
}

/// Load fixture JSON files and insert them as documents.
async fn load_fixtures(
    pool: &sqlx::PgPool,
    registry: &DocTypeRegistry,
    fixtures_dir: &Path,
) -> anyhow::Result<usize> {
    let mut count = 0;
    let entries = std::fs::read_dir(fixtures_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let docs: Vec<serde_json::Value> = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to parse fixture {:?}: {}", path, e);
                continue;
            }
        };

        // Derive doctype name from filename (e.g., "todo_category.json" → "Todo Category")
        let file_stem = path
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Try to find a matching DocType: exact match, then title-cased
        let doctype_name = if registry.exists(&file_stem).await {
            file_stem.clone()
        } else {
            // Convert snake_case to Title Case
            let title = file_stem
                .split('_')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => {
                            f.to_uppercase().collect::<String>() + c.as_str()
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            title
        };

        let meta = match registry.get_meta(&doctype_name).await {
            Ok(m) => m,
            Err(_) => {
                tracing::warn!(
                    "No DocType '{}' found for fixture {:?}, skipping",
                    doctype_name,
                    path
                );
                continue;
            }
        };

        for mut doc in docs {
            // Skip if doc already exists (by id)
            if let Some(name) = doc.get("id").and_then(|v| v.as_str()) {
                let exists = crud::get_doc(pool, &meta, name).await.is_ok();
                if exists {
                    tracing::debug!("Fixture '{}' already exists, skipping", name);
                    continue;
                }
            }

            match crud::insert_doc(pool, &meta, &mut doc, "Administrator").await {
                Ok(_) => count += 1,
                Err(e) => {
                    tracing::warn!("Failed to insert fixture: {}", e);
                }
            }
        }
    }

    Ok(count)
}
