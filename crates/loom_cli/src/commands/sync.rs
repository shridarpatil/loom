//! Shared sync logic used by both `migrate` and `install-app`.

use sqlx::PgPool;
use std::path::Path;

use loom_core::doctype::{DocTypeRegistry, Meta, crud};
use loom_core::db::migrate::{migrate_all, migrate_system_tables, migrate_doctype, seed_admin_to_doctype_table};

/// Run full migration: system tables, DocTypes, seeds, app configs, scripts.
pub async fn run_full_migrate(
    pool: &PgPool,
    apps_dir: &Path,
) -> anyhow::Result<()> {
    // System tables
    migrate_system_tables(pool).await?;
    tracing::info!("System tables ready");

    // Registry
    let registry = DocTypeRegistry::new();
    let doctype_meta = Meta::doctype_meta();
    migrate_doctype(pool, &doctype_meta).await?;
    registry.register(doctype_meta).await;

    // Load DB first, then filesystem overrides
    let db_count = registry.load_from_database(pool).await?;
    if db_count > 0 {
        tracing::info!("Loaded {} DocTypes from database", db_count);
    }

    let core_dir = Path::new("core_doctypes");
    if core_dir.exists() {
        let count = registry.load_from_directory(core_dir).await?;
        if count > 0 {
            tracing::info!("Loaded {} core DocTypes", count);
        }
    }

    if apps_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(apps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let doctypes_dir = path.join("doctypes");
                    if doctypes_dir.exists() {
                        let count = registry.load_from_directory(&doctypes_dir).await?;
                        if count > 0 {
                            tracing::info!(
                                "Loaded {} DocTypes from '{}'",
                                count,
                                path.file_name().unwrap_or_default().to_string_lossy()
                            );
                        }
                    }
                }
            }
        }
    }

    // DDL migrations
    migrate_all(pool, &registry).await?;

    // Seeds
    seed_admin_to_doctype_table(pool).await?;

    // Sync registry → DB
    registry.sync_to_database(pool).await?;

    // Refresh app configs from filesystem
    refresh_app_configs(pool, apps_dir).await?;

    // Reload scripts
    reload_scripts(pool, apps_dir).await?;

    // Reload client scripts
    reload_client_scripts(pool, apps_dir).await?;

    Ok(())
}

/// Load fixture files from an app's fixtures/ directory.
pub async fn load_fixtures(
    pool: &PgPool,
    apps_dir: &Path,
) -> anyhow::Result<usize> {
    let registry = DocTypeRegistry::new();
    registry.load_from_database(pool).await?;

    let mut total = 0;
    if !apps_dir.exists() { return Ok(0); }

    let entries = std::fs::read_dir(apps_dir)?;
    for entry in entries.flatten() {
        let fixtures_dir = entry.path().join("fixtures");
        if !fixtures_dir.exists() { continue; }

        let dir_entries = std::fs::read_dir(&fixtures_dir)?;
        for de in dir_entries.flatten() {
            let path = de.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = std::fs::read_to_string(&path)?;
                let docs: Vec<serde_json::Value> = match serde_json::from_str(&content) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("Failed to parse fixture {:?}: {}", path, e);
                        continue;
                    }
                };

                let doctype_name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .replace('_', " ");

                // Try exact name first, then title case
                let meta = match registry.get_meta(&doctype_name).await {
                    Ok(m) => m,
                    Err(_) => {
                        let title = to_title_case(&doctype_name);
                        match registry.get_meta(&title).await {
                            Ok(m) => m,
                            Err(_) => {
                                tracing::warn!("No DocType found for fixture {:?}, skipping", path);
                                continue;
                            }
                        }
                    }
                };

                for mut doc in docs {
                    if let Some(name) = doc.get("id").and_then(|v| v.as_str()) {
                        if crud::get_doc(pool, &meta, name).await.is_ok() {
                            continue; // Already exists
                        }
                    }
                    match crud::insert_doc(pool, &meta, &mut doc, "Administrator").await {
                        Ok(_) => total += 1,
                        Err(e) => tracing::warn!("Failed to insert fixture: {}", e),
                    }
                }
            }
        }
    }
    Ok(total)
}

/// Re-read loom_app.toml + hooks.toml from all apps and update installed_apps config.
pub async fn refresh_app_configs(pool: &PgPool, apps_dir: &Path) -> anyhow::Result<()> {
    if !apps_dir.exists() { return Ok(()); }

    // Keep system app, refresh everything else
    let existing: serde_json::Value = sqlx::query_scalar(
        "SELECT value FROM \"__site_config\" WHERE key = 'installed_apps'",
    )
    .fetch_optional(pool)
    .await?
    .unwrap_or(serde_json::json!([]));

    let mut apps: Vec<serde_json::Value> = existing
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|a| a.get("name").and_then(|v| v.as_str()) == Some("system"))
        .collect();

    let entries = std::fs::read_dir(apps_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }

        let toml_path = path.join("loom_app.toml");
        if !toml_path.exists() { continue; }

        let content = std::fs::read_to_string(&toml_path)?;
        let parsed: toml::Value = content.parse()?;
        let app_section = match parsed.get("app") {
            Some(a) => a,
            None => continue,
        };

        let name = app_section.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if name.is_empty() { continue; }

        let hooks_path = path.join("hooks.toml");
        let (workspace, dashboard) = if hooks_path.exists() {
            let hooks_content = std::fs::read_to_string(&hooks_path).unwrap_or_default();
            let hooks: toml::Value = hooks_content.parse().unwrap_or(toml::Value::Table(Default::default()));
            (
                parse_toml_array(&hooks, "workspace", &["label", "route", "icon"]),
                parse_toml_array(&hooks, "dashboard", &["type", "label", "doctype", "route", "color"]),
            )
        } else {
            (vec![], vec![])
        };

        apps.push(serde_json::json!({
            "name": name,
            "title": app_section.get("title").and_then(|v| v.as_str()).unwrap_or(&name),
            "icon": app_section.get("icon").and_then(|v| v.as_str()),
            "color": app_section.get("color").and_then(|v| v.as_str()),
            "modules": app_section.get("modules").and_then(|v| v.as_array()),
            "workspace": workspace,
            "dashboard": dashboard,
        }));

        tracing::info!("Refreshed config for '{}'", name);
    }

    sqlx::query(
        "INSERT INTO \"__site_config\" (key, value, modified) VALUES ('installed_apps', $1, NOW()) \
         ON CONFLICT (key) DO UPDATE SET value = $1, modified = NOW()",
    )
    .bind(&serde_json::json!(apps))
    .execute(pool)
    .await?;

    Ok(())
}

/// Reload Rhai scripts from all apps.
pub async fn reload_scripts(pool: &PgPool, apps_dir: &Path) -> anyhow::Result<()> {
    if !apps_dir.exists() { return Ok(()); }

    let entries = std::fs::read_dir(apps_dir)?;
    for entry in entries.flatten() {
        let doctypes_dir = entry.path().join("doctypes");
        if !doctypes_dir.exists() { continue; }

        let mut stack = vec![doctypes_dir];
        while let Some(current) = stack.pop() {
            let dir_entries = std::fs::read_dir(&current)?;
            for de in dir_entries.flatten() {
                let path = de.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rhai") {
                    let slug = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    if slug.is_empty() { continue; }
                    let source = std::fs::read_to_string(&path)?;
                    sqlx::query(
                        "INSERT INTO \"__script\" (name, doctype, script, modified) \
                         VALUES ($1, $2, $3, NOW()) \
                         ON CONFLICT (name) DO UPDATE SET script = $3, doctype = $2, modified = NOW()",
                    )
                    .bind(&slug).bind(&slug).bind(&source)
                    .execute(pool).await?;
                }
            }
        }
    }
    Ok(())
}

/// Reload client scripts (.client.js) from all apps.
pub async fn reload_client_scripts(pool: &PgPool, apps_dir: &Path) -> anyhow::Result<()> {
    if !apps_dir.exists() { return Ok(()); }

    let entries = std::fs::read_dir(apps_dir)?;
    for entry in entries.flatten() {
        let doctypes_dir = entry.path().join("doctypes");
        if !doctypes_dir.exists() { continue; }

        let mut stack = vec![doctypes_dir];
        while let Some(current) = stack.pop() {
            let dir_entries = std::fs::read_dir(&current)?;
            for de in dir_entries.flatten() {
                let path = de.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.to_string_lossy().ends_with(".client.js") {
                    let slug = path.file_name()
                        .and_then(|s| s.to_str())
                        .and_then(|s| s.strip_suffix(".client.js"))
                        .unwrap_or("").to_string();
                    if slug.is_empty() { continue; }

                    let json_path = path.parent().unwrap().join(format!("{}.json", slug));
                    let doctype_name = if json_path.exists() {
                        std::fs::read_to_string(&json_path).ok()
                            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                            .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                            .unwrap_or(slug.clone())
                    } else {
                        slug.clone()
                    };

                    let source = std::fs::read_to_string(&path)?;
                    sqlx::query(
                        "INSERT INTO \"__customization\" (doctype, overrides, client_script, modified) \
                         VALUES ($1, '{}', $2, NOW()) \
                         ON CONFLICT (doctype) DO UPDATE SET client_script = $2, modified = NOW()",
                    )
                    .bind(&doctype_name).bind(&source)
                    .execute(pool).await?;
                }
            }
        }
    }
    Ok(())
}

fn parse_toml_array(hooks: &toml::Value, key: &str, fields: &[&str]) -> Vec<serde_json::Value> {
    hooks.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|item| {
                let table = item.as_table()?;
                let mut obj = serde_json::Map::new();
                for &field in fields {
                    if let Some(val) = table.get(field) {
                        if let Some(s) = val.as_str() {
                            obj.insert(field.to_string(), serde_json::json!(s));
                        }
                    }
                }
                if obj.is_empty() { None } else { Some(serde_json::Value::Object(obj)) }
            }).collect()
        })
        .unwrap_or_default()
}

fn to_title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
