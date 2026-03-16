use clap::Args;
use sqlx::postgres::PgPoolOptions;

use loom_core::db::migrate::migrate_system_tables;
use loom_core::doctype::{DocTypeRegistry, Meta, crud};

#[derive(Debug, Args)]
pub struct ExportFixturesArgs {
    /// DocType to export (omit to export all fixtures declared in hooks.toml)
    #[arg(short, long)]
    pub doctype: Option<String>,

    /// JSON filters, e.g. '{"module":"HR"}' or '[["status","=","Active"]]'
    #[arg(short, long)]
    pub filters: Option<String>,

    /// Target app directory name
    #[arg(short, long)]
    pub app: Option<String>,

    /// Database URL (overrides site config)
    #[arg(long, env = "DATABASE_URL")]
    pub db_url: Option<String>,

    /// Site name
    #[arg(long)]
    pub site: Option<String>,
}

/// A fixture declaration from hooks.toml
#[derive(Debug)]
struct FixtureDecl {
    doctype: String,
    filters: Option<serde_json::Value>,
    app: String,
}

pub async fn run(args: ExportFixturesArgs) -> anyhow::Result<()> {
    let db_url = crate::site_config::resolve_db_url(args.db_url, args.site.as_deref());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    migrate_system_tables(&pool).await?;

    // Load registry
    let registry = DocTypeRegistry::new();
    registry.register(Meta::doctype_meta()).await;

    let core_dir = std::path::Path::new("core_doctypes");
    if core_dir.exists() {
        registry.load_from_directory(core_dir).await?;
    }

    let apps_dir = std::path::Path::new("apps");
    if apps_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(apps_dir) {
            for entry in entries.flatten() {
                let doctypes_dir = entry.path().join("doctypes");
                if doctypes_dir.exists() {
                    registry.load_from_directory(&doctypes_dir).await?;
                }
            }
        }
    }

    registry.load_from_database(&pool).await?;

    if let Some(ref doctype) = args.doctype {
        // Single DocType export
        let filters: Option<serde_json::Value> = args
            .filters
            .as_deref()
            .and_then(|f| serde_json::from_str(f).ok());

        let meta = registry.get_meta(doctype).await?;
        let app = args.app.unwrap_or_else(|| {
            find_app_for_doctype(apps_dir, &doctype.to_lowercase().replace(' ', "_"), &meta.module)
                .unwrap_or_else(|| "unknown".to_string())
        });

        export_one(&pool, &registry, doctype, filters.as_ref(), &app, apps_dir).await?;
    } else {
        // Export all fixtures declared in hooks.toml across all apps
        let decls = load_fixture_declarations(apps_dir);
        if decls.is_empty() {
            println!("No [[fixtures]] declared in any app's hooks.toml.");
            println!("Usage: loom export-fixtures --doctype Role --filters '{{\"module\":\"HR\"}}'");
            println!("Or add to hooks.toml:");
            println!("  [[fixtures]]");
            println!("  doctype = \"Role\"");
            println!("  filters = {{module = \"HR\"}}");
            return Ok(());
        }

        for decl in &decls {
            export_one(&pool, &registry, &decl.doctype, decl.filters.as_ref(), &decl.app, apps_dir).await?;
        }
    }

    Ok(())
}

async fn export_one(
    pool: &sqlx::PgPool,
    registry: &DocTypeRegistry,
    doctype: &str,
    filters: Option<&serde_json::Value>,
    app: &str,
    apps_dir: &std::path::Path,
) -> anyhow::Result<()> {
    let meta = registry.get_meta(doctype).await?;

    let docs = crud::get_list(pool, &meta, filters, None, None, Some(10000), None, None).await?;

    if docs.is_empty() {
        println!("  {} — no records match", doctype);
        return Ok(());
    }

    let slug = doctype.to_lowercase().replace(' ', "_");
    let fixtures_dir = apps_dir.join(app).join("fixtures");
    std::fs::create_dir_all(&fixtures_dir)?;

    let file_path = fixtures_dir.join(format!("{}.json", slug));
    let pretty = serde_json::to_string_pretty(&docs)?;
    std::fs::write(&file_path, &pretty)?;

    println!("  {} — {} record(s) → apps/{}/fixtures/{}.json", doctype, docs.len(), app, slug);
    Ok(())
}

/// Read [[fixtures]] from all apps/*/hooks.toml
fn load_fixture_declarations(apps_dir: &std::path::Path) -> Vec<FixtureDecl> {
    let mut decls = Vec::new();

    let entries = match std::fs::read_dir(apps_dir) {
        Ok(e) => e,
        Err(_) => return decls,
    };

    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let app_name = entry.file_name().to_str().unwrap_or("").to_string();
        let hooks_path = entry.path().join("hooks.toml");
        if !hooks_path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&hooks_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let hooks: toml::Value = match content.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(fixtures) = hooks.get("fixtures").and_then(|v| v.as_array()) {
            for fixture in fixtures {
                if let Some(table) = fixture.as_table() {
                    let doctype = table
                        .get("doctype")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if doctype.is_empty() {
                        continue;
                    }

                    // Parse filters from TOML table to JSON
                    let filters = table.get("filters").and_then(|v| {
                        // Convert toml::Value filters to serde_json::Value
                        let json_str = serde_json::to_string(&v).ok()?;
                        serde_json::from_str(&json_str).ok()
                    });

                    decls.push(FixtureDecl {
                        doctype,
                        filters,
                        app: app_name.clone(),
                    });
                }
            }
        }
    }

    decls
}

fn find_app_for_doctype(
    apps_dir: &std::path::Path,
    slug: &str,
    module: &str,
) -> Option<String> {
    // Check which app has this doctype
    if let Ok(entries) = std::fs::read_dir(apps_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let dt_dir = entry.path().join("doctypes").join(slug);
            if dt_dir.is_dir() {
                return entry.file_name().to_str().map(|s| s.to_string());
            }
        }
    }

    // Try module name as app name
    let module_slug = module.to_lowercase().replace(' ', "_");
    if apps_dir.join(&module_slug).is_dir() {
        return Some(module_slug);
    }

    // Single app fallback
    let entries: Vec<_> = std::fs::read_dir(apps_dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    if entries.len() == 1 {
        return entries[0].file_name().to_str().map(|s| s.to_string());
    }

    None
}
