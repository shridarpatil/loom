use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use loom_core::context::RequestContext;
use crate::server::AppState;

/// GET /api/customize/:doctype — get customization for a DocType
pub async fn get_customization(
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let row: Option<(Value, String)> = sqlx::query_as(
        "SELECT overrides, COALESCE(client_script, '') FROM \"__customization\" WHERE doctype = $1",
    )
    .bind(&doctype)
    .fetch_optional(ctx.pool())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    // Load server script from __script table
    let slug = doctype.to_lowercase().replace(' ', "_");
    let server_script: String = sqlx::query_scalar(
        "SELECT script FROM \"__script\" WHERE name = $1 OR doctype = $2",
    )
    .bind(&slug)
    .bind(&doctype)
    .fetch_optional(ctx.pool())
    .await
    .ok()
    .flatten()
    .unwrap_or_default();

    match row {
        Some((overrides, client_script)) => Ok(Json(json!({
            "data": {
                "doctype": doctype,
                "overrides": overrides,
                "client_script": client_script,
                "server_script": server_script,
            }
        }))),
        None => Ok(Json(json!({
            "data": {
                "doctype": doctype,
                "overrides": {},
                "client_script": "",
                "server_script": server_script,
            }
        }))),
    }
}

/// PUT /api/customize/:doctype — save customization (field overrides + client script + server script)
pub async fn save_customization(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let overrides = body.get("overrides").cloned().unwrap_or(json!({}));
    let client_script = body
        .get("client_script")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let server_script = body
        .get("server_script")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Save field overrides + client script to __customization
    sqlx::query(
        "INSERT INTO \"__customization\" (doctype, overrides, client_script, modified) \
         VALUES ($1, $2, $3, NOW()) \
         ON CONFLICT (doctype) DO UPDATE SET overrides = $2, client_script = $3, modified = NOW()",
    )
    .bind(&doctype)
    .bind(&overrides)
    .bind(&client_script)
    .execute(ctx.pool())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    // Save server script to __script table and hot-reload hook runner
    let slug = doctype.to_lowercase().replace(' ', "_");
    if !server_script.is_empty() {
        sqlx::query(
            "INSERT INTO \"__script\" (name, doctype, script, modified) \
             VALUES ($1, $2, $3, NOW()) \
             ON CONFLICT (name) DO UPDATE SET script = $3, doctype = $2, modified = NOW()",
        )
        .bind(&slug)
        .bind(&doctype)
        .bind(&server_script)
        .execute(ctx.pool())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

        // Hot-reload: update the hook runner with the new script
        state.hook_runner.load_script(&slug, server_script.clone()).await;
        tracing::info!("Hot-reloaded server script for '{}'", doctype);
    } else {
        // Clear script if empty
        sqlx::query("DELETE FROM \"__script\" WHERE name = $1")
            .bind(&slug)
            .execute(ctx.pool())
            .await
            .ok();
    }

    state.cache.invalidate_customization(&doctype).await;
    tracing::info!("Saved customization for '{}'", doctype);

    Ok(Json(json!({
        "message": "Customization saved",
        "data": {
            "doctype": doctype,
            "overrides": overrides,
            "client_script": client_script,
            "server_script": server_script,
        }
    })))
}

/// POST /api/customize/:doctype/export — export customization to app directory
pub async fn export_customization(
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let row: Option<(Value, String)> = sqlx::query_as(
        "SELECT overrides, COALESCE(client_script, '') FROM \"__customization\" WHERE doctype = $1",
    )
    .bind(&doctype)
    .fetch_optional(ctx.pool())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    let slug = doctype.to_lowercase().replace(' ', "_");

    // Load server script
    let server_script: String = sqlx::query_scalar(
        "SELECT script FROM \"__script\" WHERE name = $1 OR doctype = $2",
    )
    .bind(&slug)
    .bind(&doctype)
    .fetch_optional(ctx.pool())
    .await
    .ok()
    .flatten()
    .unwrap_or_default();

    let (overrides, client_script) = row.unwrap_or((json!({}), String::new()));

    if overrides.as_object().is_some_and(|o| o.is_empty())
        && client_script.is_empty()
        && server_script.is_empty()
    {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "No customization found for this DocType"})),
        ));
    }

    // Find app directory
    let apps_dir = super::doctype::find_apps_dir();
    let app_name = super::doctype::find_app_for_doctype(&apps_dir, &slug)
        .or_else(|| super::doctype::find_single_app(&apps_dir))
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Could not determine target app"})),
            )
        })?;

    let dt_dir = apps_dir.join(&app_name).join("doctypes").join(&slug);
    std::fs::create_dir_all(&dt_dir).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to create directory: {}", e)})))
    })?;

    let mut exported = Vec::new();

    // Export field overrides
    if overrides.as_object().is_some_and(|o| !o.is_empty()) {
        let path = dt_dir.join(format!("{}.customize.json", slug));
        let pretty = serde_json::to_string_pretty(&overrides).unwrap();
        std::fs::write(&path, &pretty).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to write: {}", e)})))
        })?;
        exported.push(format!("{}.customize.json", slug));
    }

    // Export client script as JS
    if !client_script.is_empty() {
        let path = dt_dir.join(format!("{}.client.js", slug));
        std::fs::write(&path, &client_script).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to write: {}", e)})))
        })?;
        exported.push(format!("{}.client.js", slug));
    }

    // Export server script as Rhai
    if !server_script.is_empty() {
        let path = dt_dir.join(format!("{}.rhai", slug));
        std::fs::write(&path, &server_script).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to write: {}", e)})))
        })?;
        exported.push(format!("{}.rhai", slug));
    }

    tracing::info!("Exported customization for '{}' to app '{}'", doctype, app_name);

    Ok(Json(json!({
        "message": format!("Exported to apps/{}/doctypes/{}: {}", app_name, slug, exported.join(", ")),
        "app": app_name,
        "files": exported,
    })))
}

/// Apply customization overrides to a Meta JSON value.
/// Uses cache if provided to avoid DB hit on every request.
pub async fn apply_customization(pool: &sqlx::PgPool, doctype: &str, meta: &mut Value, cache: Option<&crate::cache::AppCache>) {
    // Try cache first
    let cached = if let Some(c) = cache {
        c.customizations.get(doctype).await
    } else {
        None
    };

    let (overrides, client_script) = if let Some(Some(cached_val)) = cached {
        // Cache hit — extract overrides and client_script
        let cs = cached_val.get("__client_script").and_then(|v| v.as_str()).unwrap_or("").to_string();
        (cached_val, cs)
    } else {
        // Cache miss or no cache — query DB
        let row: Option<(Value, String)> = sqlx::query_as(
            "SELECT overrides, COALESCE(client_script, '') FROM \"__customization\" WHERE doctype = $1",
        )
        .bind(doctype)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        let Some((overrides, client_script)) = row else {
            // Store negative cache
            if let Some(c) = cache {
                c.customizations.set(doctype.to_string(), None).await;
            }
            return;
        };

        // Store in cache (embed client_script in the overrides for caching)
        if let Some(c) = cache {
            let mut cache_val = overrides.clone();
            if let Some(obj) = cache_val.as_object_mut() {
                obj.insert("__client_script".to_string(), json!(client_script));
            }
            c.customizations.set(doctype.to_string(), Some(cache_val)).await;
        }

        (overrides, client_script)
    };

    if let Some(field_overrides) = overrides.as_object() {
        if let Some(fields) = meta.get_mut("fields").and_then(|f| f.as_array_mut()) {
            for field in fields.iter_mut() {
                let fieldname = field.get("fieldname").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(patches) = field_overrides.get(fieldname).and_then(|v| v.as_object()) {
                    if let Some(obj) = field.as_object_mut() {
                        for (key, val) in patches {
                            obj.insert(key.clone(), val.clone());
                        }
                    }
                }
            }
        }
    }

    // Apply permission overrides: merge with defaults using shared function
    if let Some(field_overrides) = overrides.as_object() {
        if let Some(perm_overrides) = field_overrides.get("__permissions") {
            if let Ok(override_perms) = serde_json::from_value::<Vec<loom_core::doctype::DocPermMeta>>(perm_overrides.clone()) {
                if let Some(default_perms_json) = meta.get("permissions") {
                    if let Ok(default_perms) = serde_json::from_value::<Vec<loom_core::doctype::DocPermMeta>>(default_perms_json.clone()) {
                        let merged = loom_core::doctype::merge_permission_overrides(&default_perms, &override_perms);
                        if let Some(obj) = meta.as_object_mut() {
                            obj.insert("permissions".to_string(), serde_json::to_value(&merged).unwrap_or_default());
                        }
                    }
                }
            }
        }
    }

    // Client script: loaded from DB (synced by migrate from .client.js files)
    if !client_script.is_empty() {
        if let Some(obj) = meta.as_object_mut() {
            obj.insert("client_script".to_string(), json!(client_script));
        }
    }
}


// =========================================================================
// Role Permission Manager API
// =========================================================================

/// GET /api/role-permission/:doctype — get default and overridden permissions
pub async fn get_role_permissions(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Get original (un-overridden) meta
    let meta = state
        .registry
        .get_meta(&doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    // Get overrides
    let row: Option<(Value,)> = sqlx::query_as(
        "SELECT overrides FROM \"__customization\" WHERE doctype = $1",
    )
    .bind(&doctype)
    .fetch_optional(ctx.pool())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    let override_permissions: Option<Value> = row
        .and_then(|(overrides,)| overrides.get("__permissions").cloned());

    // Get all distinct roles
    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT jsonb_array_elements_text(roles) AS role FROM \"__user\" ORDER BY role",
    )
    .fetch_all(ctx.pool())
    .await
    .unwrap_or_default();

    Ok(Json(json!({
        "data": {
            "doctype": doctype,
            "is_submittable": meta.is_submittable,
            "default_permissions": meta.permissions,
            "override_permissions": override_permissions,
            "roles": roles,
        }
    })))
}

/// PUT /api/role-permission/:doctype — save permission overrides
pub async fn save_role_permissions(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let permissions = body.get("permissions").ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Missing 'permissions' field"})),
        )
    })?;

    // Load existing overrides to preserve field overrides
    let existing: Value = sqlx::query_scalar(
        "SELECT overrides FROM \"__customization\" WHERE doctype = $1",
    )
    .bind(&doctype)
    .fetch_optional(ctx.pool())
    .await
    .ok()
    .flatten()
    .unwrap_or(json!({}));

    let mut overrides = existing.as_object().cloned().unwrap_or_default();
    overrides.insert("__permissions".to_string(), permissions.clone());

    sqlx::query(
        "INSERT INTO \"__customization\" (doctype, overrides, modified) \
         VALUES ($1, $2, NOW()) \
         ON CONFLICT (doctype) DO UPDATE SET overrides = $2, modified = NOW()",
    )
    .bind(&doctype)
    .bind(&json!(overrides))
    .execute(ctx.pool())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    state.cache.invalidate_customization(&doctype).await;
    tracing::info!("Saved permission overrides for '{}'", doctype);

    Ok(Json(json!({ "message": "Permissions saved" })))
}

/// DELETE /api/role-permission/:doctype — reset permissions to defaults
pub async fn reset_role_permissions(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Load existing overrides, remove __permissions, save back
    let existing: Value = sqlx::query_scalar(
        "SELECT overrides FROM \"__customization\" WHERE doctype = $1",
    )
    .bind(&doctype)
    .fetch_optional(ctx.pool())
    .await
    .ok()
    .flatten()
    .unwrap_or(json!({}));

    let mut overrides = existing.as_object().cloned().unwrap_or_default();
    overrides.remove("__permissions");

    if overrides.is_empty() {
        // No overrides left — delete the row
        sqlx::query("DELETE FROM \"__customization\" WHERE doctype = $1")
            .bind(&doctype)
            .execute(ctx.pool())
            .await
            .ok();
    } else {
        sqlx::query(
            "UPDATE \"__customization\" SET overrides = $1, modified = NOW() WHERE doctype = $2",
        )
        .bind(&json!(overrides))
        .bind(&doctype)
        .execute(ctx.pool())
        .await
        .ok();
    }

    state.cache.invalidate_customization(&doctype).await;
    tracing::info!("Reset permission overrides for '{}'", doctype);

    Ok(Json(json!({ "message": "Permissions reset to defaults" })))
}

/// GET /api/role-permission-by-role/:role — get all DocType permissions for a given role
pub async fn get_permissions_by_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(role): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let all_doctypes = state.registry.all_doctypes().await;
    let mut entries: Vec<Value> = Vec::new();

    for dt_name in &all_doctypes {
        let meta = match state.registry.get_effective_meta(dt_name, ctx.pool()).await {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Skip child tables — they inherit permissions from parent
        if meta.is_child_table {
            continue;
        }

        // Find permission rules for this role
        let role_perms: Vec<&loom_core::doctype::DocPermMeta> = meta
            .permissions
            .iter()
            .filter(|p| p.role == role)
            .collect();

        if role_perms.is_empty() {
            continue;
        }

        for perm in role_perms {
            entries.push(json!({
                "doctype": dt_name,
                "is_submittable": meta.is_submittable,
                "role": perm.role,
                "permlevel": perm.permlevel,
                "read": perm.read,
                "write": perm.write,
                "create": perm.create,
                "delete": perm.delete,
                "submit": perm.submit,
                "cancel": perm.cancel,
                "if_owner": perm.if_owner,
            }));
        }
    }

    // Sort by doctype name
    entries.sort_by(|a, b| {
        let da = a.get("doctype").and_then(|v| v.as_str()).unwrap_or("");
        let db = b.get("doctype").and_then(|v| v.as_str()).unwrap_or("");
        da.cmp(db)
    });

    // Get all distinct roles
    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT jsonb_array_elements_text(roles) AS role FROM \"__user\" ORDER BY role",
    )
    .fetch_all(ctx.pool())
    .await
    .unwrap_or_default();

    Ok(Json(json!({
        "data": {
            "role": role,
            "permissions": entries,
            "roles": roles,
        }
    })))
}
