use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::server::AppState;

/// GET /api/settings/:key — returns current user's setting
pub async fn get_setting(
    State(state): State<AppState>,
    Extension(ctx): Extension<loom_core::context::RequestContext>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT value FROM \"__user_settings\" WHERE user_email = $1 AND key = $2"
    )
    .bind(&ctx.user)
    .bind(&key)
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(value)) => Json(serde_json::json!({ "data": value })).into_response(),
        Ok(None) => Json(serde_json::json!({ "data": null })).into_response(),
        Err(e) => {
            tracing::error!("Failed to read setting: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to read setting"
            }))).into_response()
        }
    }
}

/// PUT /api/settings/:key — saves current user's setting
pub async fn save_setting(
    State(state): State<AppState>,
    Extension(ctx): Extension<loom_core::context::RequestContext>,
    Path(key): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let result = sqlx::query(
        "INSERT INTO \"__user_settings\" (user_email, key, value, modified)
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (user_email, key) DO UPDATE SET value = $3, modified = NOW()"
    )
    .bind(&ctx.user)
    .bind(&key)
    .bind(&body)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({ "message": "Setting saved" })).into_response(),
        Err(e) => {
            tracing::error!("Failed to save setting: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to save setting"
            }))).into_response()
        }
    }
}

/// GET /api/sidebar — returns sidebar configuration
/// Merges DocTypes by module + user overrides (pinned/hidden)
pub async fn get_sidebar(
    State(state): State<AppState>,
    Extension(ctx): Extension<loom_core::context::RequestContext>,
) -> impl IntoResponse {
    // 1. Load all DocTypes from registry, group by module
    let doctypes = state.registry.all_doctypes().await;

    let mut module_map: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
        std::collections::BTreeMap::new();

    for dt_name in &doctypes {
        if let Ok(meta) = state.registry.get_meta(dt_name).await {
            let module = if meta.module.is_empty() { "Core".to_string() } else { meta.module.clone() };
            module_map.entry(module.clone()).or_default().push(serde_json::json!({
                "name": dt_name,
                "route": format!("/app/{}", dt_name),
                "module": module,
            }));
        }
    }

    let sections: Vec<serde_json::Value> = module_map
        .into_iter()
        .map(|(module, items)| {
            serde_json::json!({
                "label": module,
                "items": items,
            })
        })
        .collect();

    // 2. Load user sidebar overrides
    let user_sidebar = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT value FROM \"__user_settings\" WHERE user_email = $1 AND key = 'sidebar'"
    )
    .bind(&ctx.user)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let pinned = user_sidebar
        .as_ref()
        .and_then(|v| v.get("pinned"))
        .cloned()
        .unwrap_or(serde_json::json!([]));

    let hidden = user_sidebar
        .as_ref()
        .and_then(|v| v.get("hidden"))
        .cloned()
        .unwrap_or(serde_json::json!([]));

    Json(serde_json::json!({
        "data": {
            "sections": sections,
            "pinned": pinned,
            "hidden": hidden,
        }
    }))
}

/// GET /api/plugins/pages — returns registered plugin pages from app hooks
pub async fn get_plugin_pages(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let mut pages: Vec<serde_json::Value> = Vec::new();

    // Scan apps/*/hooks.toml for [[pages]]
    let apps_dir = find_apps_dir();
    if let Some(dir) = apps_dir {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let hooks_path = entry.path().join("hooks.toml");
                if hooks_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&hooks_path) {
                        if let Ok(hooks) = content.parse::<toml::Value>() {
                            if let Some(page_list) = hooks.get("pages").and_then(|v| v.as_array()) {
                                for page in page_list {
                                    if let Some(table) = page.as_table() {
                                        pages.push(serde_json::json!({
                                            "route": table.get("route").and_then(|v| v.as_str()).unwrap_or(""),
                                            "label": table.get("label").and_then(|v| v.as_str()).unwrap_or(""),
                                            "bundle": table.get("bundle").and_then(|v| v.as_str()).unwrap_or(""),
                                            "component": table.get("component").and_then(|v| v.as_str()).unwrap_or(""),
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Json(serde_json::json!({ "data": pages }))
}

/// Find the apps/ directory relative to the project root
fn find_apps_dir() -> Option<std::path::PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    // Try cwd/apps, then parent/apps, etc.
    let mut dir = cwd.as_path();
    loop {
        let apps = dir.join("apps");
        if apps.is_dir() {
            return Some(apps);
        }
        dir = dir.parent()?;
    }
}
