use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::server::AppState;
use super::customize::apply_customization;

/// GET /api/doctype/:name — return DocType meta JSON with customizations applied
pub async fn get_meta(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let meta = state
        .registry
        .get_meta(&name)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    let mut meta_json = serde_json::to_value(&meta).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    // Merge site-level customizations (hidden fields, property changes, client scripts)
    apply_customization(&state.pool, &name, &mut meta_json, Some(&state.cache)).await;

    Ok(Json(json!({ "data": meta_json })))
}

#[derive(Debug, Deserialize)]
pub struct ExportParams {
    pub app: Option<String>,
}

/// POST /api/doctype/:name/export — write DocType JSON into an app's doctypes/ directory
/// Requires developer mode.
pub async fn export_meta(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<ExportParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !is_developer_mode() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Export is only available in developer mode. Set developer_mode in site config."})),
        ));
    }

    let meta = state
        .registry
        .get_meta(&name)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    let meta_json = serde_json::to_value(&meta).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    let slug = name.to_lowercase().replace(' ', "_");
    let filename = format!("{}.json", slug);
    let apps_dir = find_apps_dir();

    // Resolve which app to export into
    let app_name = if let Some(app) = params.app {
        // Explicit app specified
        let app_path = apps_dir.join(&app);
        if !app_path.is_dir() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("App '{}' not found in {}", app, apps_dir.display())})),
            ));
        }
        app
    } else {
        // Auto-detect: find which app already has this DocType
        find_app_for_doctype(&apps_dir, &slug)
            .or_else(|| find_single_app(&apps_dir))
            .ok_or_else(|| {
                let apps = list_apps(&apps_dir);
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "Multiple apps found. Specify which app to export to.",
                        "apps": apps
                    })),
                )
            })?
    };

    let dir_path = apps_dir.join(&app_name).join("doctypes").join(&slug);
    std::fs::create_dir_all(&dir_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create directory: {}", e)})),
        )
    })?;

    let file_path = dir_path.join(&filename);
    let pretty = serde_json::to_string_pretty(&meta_json).unwrap();
    std::fs::write(&file_path, &pretty).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to write file: {}", e)})),
        )
    })?;

    let abs_path = std::fs::canonicalize(&file_path).unwrap_or_else(|_| file_path.clone());

    tracing::info!("Exported DocType '{}' to {:?}", name, abs_path);

    Ok(Json(json!({
        "message": format!("Exported to apps/{}/doctypes/{}/{}", app_name, slug, filename),
        "path": abs_path.to_string_lossy(),
        "app": app_name
    })))
}

/// GET /api/apps — list installed apps with metadata (icon, color, modules)
pub async fn list_apps_handler(
    State(state): State<AppState>,
) -> Json<Value> {
    // Try to get rich metadata from __site_config
    let installed: Option<Value> = sqlx::query_scalar(
        "SELECT value FROM \"__site_config\" WHERE key = 'installed_apps'",
    )
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    if let Some(apps) = installed {
        return Json(json!({ "data": apps }));
    }

    // Fallback: scan apps directory and read loom_app.toml for each
    let apps_dir = find_apps_dir();
    let mut apps: Vec<Value> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&apps_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() { continue; }
            let toml_path = entry.path().join("loom_app.toml");
            if let Ok(content) = std::fs::read_to_string(&toml_path) {
                if let Ok(parsed) = content.parse::<toml::Value>() {
                    if let Some(app) = parsed.get("app") {
                        apps.push(json!({
                            "name": app.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                            "title": app.get("title").and_then(|v| v.as_str()).unwrap_or_else(||
                                app.get("name").and_then(|v| v.as_str()).unwrap_or("")
                            ),
                            "icon": app.get("icon").and_then(|v| v.as_str()),
                            "color": app.get("color").and_then(|v| v.as_str()),
                            "modules": app.get("modules").and_then(|v| v.as_array()),
                        }));
                    }
                }
            }
        }
    }
    Json(json!({ "data": apps }))
}

/// Find the project root's apps/ directory.
pub fn find_apps_dir() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        if dir.join("apps").is_dir() && dir.join("Cargo.toml").exists() {
            return dir.join("apps");
        }
        if !dir.pop() {
            break;
        }
    }
    // Fallback to cwd/apps
    std::env::current_dir().unwrap_or_default().join("apps")
}

/// Scan apps/*/doctypes/ to find which app already contains this DocType.
pub fn find_app_for_doctype(apps_dir: &std::path::Path, slug: &str) -> Option<String> {
    let entries = std::fs::read_dir(apps_dir).ok()?;
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let dt_dir = entry.path().join("doctypes").join(slug);
        if dt_dir.is_dir() {
            return entry.file_name().to_str().map(|s| s.to_string());
        }
    }
    None
}

/// If there's exactly one app, return it.
pub fn find_single_app(apps_dir: &std::path::Path) -> Option<String> {
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

/// List all app names under apps/.
fn list_apps(apps_dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(apps_dir)
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Check if developer mode is enabled in site_config.json.
/// Looks for `sites/*/site_config.json` with `"developer_mode": true`.
pub fn is_developer_mode() -> bool {
    let sites_dir = find_sites_dir();
    if let Ok(entries) = std::fs::read_dir(&sites_dir) {
        for entry in entries.flatten() {
            let config_path = entry.path().join("site_config.json");
            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        match config.get("developer_mode") {
                            Some(serde_json::Value::Bool(true)) => return true,
                            Some(serde_json::Value::Number(n)) if n.as_i64() == Some(1) => return true,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    false
}

/// Find the sites/ directory.
fn find_sites_dir() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        if dir.join("sites").is_dir() {
            return dir.join("sites");
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir().unwrap_or_default().join("sites")
}

/// Auto-export a DocType JSON to the appropriate directory.
/// Core module DocTypes go to `core_doctypes/`, others go to `apps/{app}/doctypes/`.
/// Only runs in developer mode. Silently skips otherwise.
pub fn auto_export_doctype(meta: &loom_core::doctype::Meta) {
    let slug = meta.name.to_lowercase().replace(' ', "_");

    // Core module DocTypes → core_doctypes/
    if meta.module == "Core" {
        let core_dir = find_core_doctypes_dir();
        let dir_path = core_dir.join(&slug);
        if std::fs::create_dir_all(&dir_path).is_err() {
            tracing::warn!("Failed to create core DocType directory: {:?}", dir_path);
            return;
        }
        write_doctype_json(&dir_path, &slug, meta);
        return;
    }

    // App DocTypes → apps/{app}/doctypes/
    let apps_dir = find_apps_dir();
    let app_name = find_app_for_doctype(&apps_dir, &slug)
        .or_else(|| {
            let module_slug = meta.module.to_lowercase().replace(' ', "_");
            if apps_dir.join(&module_slug).is_dir() {
                Some(module_slug)
            } else {
                find_single_app(&apps_dir)
            }
        });

    let app_name = match app_name {
        Some(a) => a,
        None => return,
    };

    let dir_path = apps_dir.join(&app_name).join("doctypes").join(&slug);
    if std::fs::create_dir_all(&dir_path).is_err() {
        tracing::warn!("Failed to create directory for DocType export: {:?}", dir_path);
        return;
    }
    write_doctype_json(&dir_path, &slug, meta);
}

fn write_doctype_json(dir: &std::path::Path, slug: &str, meta: &loom_core::doctype::Meta) {
    let file_path = dir.join(format!("{}.json", slug));
    let meta_json = match serde_json::to_value(meta) {
        Ok(v) => v,
        Err(_) => return,
    };
    let pretty = serde_json::to_string_pretty(&meta_json).unwrap_or_default();
    match std::fs::write(&file_path, &pretty) {
        Ok(_) => tracing::info!("Auto-exported DocType '{}' to {:?}", meta.name, file_path),
        Err(e) => tracing::warn!("Failed to auto-export DocType '{}': {}", meta.name, e),
    }
}

/// Find the core_doctypes/ directory.
fn find_core_doctypes_dir() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        if dir.join("core_doctypes").is_dir() {
            return dir.join("core_doctypes");
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir().unwrap_or_default().join("core_doctypes")
}
