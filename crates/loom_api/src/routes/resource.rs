use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use loom_core::context::RequestContext;
use loom_core::db::migrate::migrate_doctype;
use loom_core::doctype::{controller, Meta};

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub filters: Option<String>, // JSON-encoded filters
    pub fields: Option<String>,  // JSON-encoded or comma-separated field names
    pub order_by: Option<String>,
    pub limit_page_length: Option<u32>,
    pub limit_start: Option<u32>,
    /// Aliases for limit_page_length / limit_start
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    /// Full-text search on name and title_field
    pub search_term: Option<String>,
}

/// GET /api/resource/:doctype — list documents
pub async fn get_list(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Special handling for listing DocTypes — use registry as source of truth
    if doctype == "DocType" {
        let all = state.registry.all_doctypes().await;
        let mut docs: Vec<Value> = Vec::new();
        for name in all {
            if let Ok(meta) = state.registry.get_meta(&name).await {
                docs.push(json!({
                    "id": meta.name,
                    "name": meta.name,
                    "module": meta.module,
                    "is_submittable": meta.is_submittable,
                    "is_child_table": meta.is_child_table,
                    "naming_rule": meta.naming_rule,
                }));
            }
        }
        return Ok(Json(json!({ "data": docs })));
    }

    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    let filters: Option<Value> = params
        .filters
        .as_deref()
        .and_then(|f| serde_json::from_str(f).ok());

    // Parse fields: try JSON array first, fall back to comma-separated
    let field_names: Option<Vec<String>> = params.fields.as_deref().and_then(|f| {
        serde_json::from_str::<Vec<String>>(f)
            .ok()
            .or_else(|| Some(f.split(',').map(|s| s.trim().to_string()).collect()))
    });
    let field_refs: Option<Vec<&str>> = field_names
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());

    // Resolve limit/offset with aliases
    let effective_limit = params.limit_page_length.or(params.limit);
    let effective_offset = params.limit_start.or(params.offset);

    let docs = controller::get_list(
        &ctx,
        &meta,
        filters.as_ref(),
        field_refs.as_deref(),
        params.order_by.as_deref(),
        effective_limit,
        effective_offset,
        params.search_term.as_deref(),
    )
    .await
    .map_err(|e| error_response(&e))?;

    Ok(Json(json!({ "data": docs })))
}

/// GET /api/resource/:doctype/:name — get a single document
pub async fn get_doc(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path((doctype, name)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    let doc = controller::get(&ctx, &meta, &name)
        .await
        .map_err(|e| error_response(&e))?;

    // No realtime event for reads
    Ok(Json(json!({ "data": doc })))
}

/// POST /api/resource/:doctype — create a new document
pub async fn insert_doc(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
    Json(mut body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    // Special handling for DocType creation
    if doctype == "DocType" {
        return create_doctype(&state, &ctx, &mut body).await;
    }

    let doc = controller::insert(&ctx, &meta, &mut body, state.hook_runner.as_ref())
        .await
        .map_err(|e| error_response(&e))?;

    let doc_name = doc.get("id").and_then(|v| v.as_str()).unwrap_or("");
    state
        .realtime
        .publish_doc_update(&doctype, doc_name, "created");

    Ok((StatusCode::CREATED, Json(json!({ "data": doc }))))
}

/// Create a new DocType: parse meta, migrate table, register, and save.
async fn create_doctype(
    state: &AppState,
    ctx: &RequestContext,
    body: &mut Value,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Build a Meta from the incoming JSON
    let new_meta: Meta = serde_json::from_value(body.clone()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid DocType definition: {}", e)})),
        )
    })?;

    if new_meta.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "DocType name is required"})),
        ));
    }

    // Check if already exists
    if state.registry.exists(&new_meta.name).await {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error": format!("DocType '{}' already exists", new_meta.name)})),
        ));
    }

    // Auto-migrate: create the database table
    migrate_doctype(ctx.pool(), &new_meta).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Migration failed: {}", e)})),
        )
    })?;

    // Save meta to __doctype table
    let meta_json = serde_json::to_value(&new_meta).unwrap();
    sqlx::query(
        "INSERT INTO \"__doctype\" (name, module, meta, modified) \
         VALUES ($1, $2, $3, NOW()) \
         ON CONFLICT (name) DO UPDATE SET meta = $3, module = $2, modified = NOW()",
    )
    .bind(&new_meta.name)
    .bind(&new_meta.module)
    .bind(&meta_json)
    .execute(ctx.pool())
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to save DocType: {}", e)})),
        )
    })?;

    // Also insert into tabDocType for CRUD consistency
    let doctype_meta = state.registry.get_meta("DocType").await.unwrap();
    let mut tab_doc = json!({
        "id": new_meta.name,
        "module": new_meta.module,
        "is_submittable": new_meta.is_submittable,
        "is_child_table": new_meta.is_child_table,
        "is_single": new_meta.is_single,
        "naming_rule": serde_json::to_value(&new_meta.naming_rule).unwrap(),
        "autoname": new_meta.autoname,
        "title_field": new_meta.title_field,
        "sort_field": new_meta.sort_field,
        "sort_order": new_meta.sort_order,
        "fields_json": serde_json::to_value(&new_meta.fields).unwrap(),
        "permissions_json": serde_json::to_value(&new_meta.permissions).unwrap(),
    });
    // Use CRUD to insert the row into tabDocType
    let _ =
        loom_core::doctype::crud::insert_doc(ctx.pool(), &doctype_meta, &mut tab_doc, &ctx.user)
            .await;

    // Auto-export JSON to app directory (developer mode only)
    if super::doctype::is_developer_mode() {
        super::doctype::auto_export_doctype(&new_meta);
    }

    // Register in the in-memory registry
    let name = new_meta.name.clone();
    state.registry.register(new_meta).await;

    tracing::info!("Created DocType '{}' with table and registry entry", name);

    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": { "name": name, "doctype": "DocType" } })),
    ))
}

/// PUT /api/resource/:doctype/:name — update a document
pub async fn update_doc(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path((doctype, name)): Path<(String, String)>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    // Special handling for DocType updates
    if doctype == "DocType" {
        return update_doctype(&state, &ctx, &name, &mut body).await;
    }

    let doc = controller::update(&ctx, &meta, &name, &mut body, state.hook_runner.as_ref())
        .await
        .map_err(|e| error_response(&e))?;

    state
        .realtime
        .publish_doc_update(&doctype, &name, "updated");

    Ok(Json(json!({ "data": doc })))
}

/// Update an existing DocType: re-parse meta, re-migrate table, update registry + DB.
async fn update_doctype(
    state: &AppState,
    ctx: &RequestContext,
    name: &str,
    body: &mut Value,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let new_meta: Meta = serde_json::from_value(body.clone()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid DocType definition: {}", e)})),
        )
    })?;

    if new_meta.name != name {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Cannot rename a DocType"})),
        ));
    }

    // Re-migrate table (adds new columns)
    migrate_doctype(ctx.pool(), &new_meta).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Migration failed: {}", e)})),
        )
    })?;

    // Update __doctype table
    let meta_json = serde_json::to_value(&new_meta).unwrap();
    sqlx::query(
        "UPDATE \"__doctype\" SET meta = $1, module = $2, modified = NOW() WHERE name = $3",
    )
    .bind(&meta_json)
    .bind(&new_meta.module)
    .bind(name)
    .execute(ctx.pool())
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to update DocType: {}", e)})),
        )
    })?;

    // Auto-export JSON to app directory (developer mode only)
    if super::doctype::is_developer_mode() {
        super::doctype::auto_export_doctype(&new_meta);
    }

    // Update in-memory registry
    let updated_name = new_meta.name.clone();
    state.registry.register(new_meta).await;

    tracing::info!("Updated DocType '{}'", updated_name);

    Ok(Json(
        json!({ "data": { "name": updated_name, "doctype": "DocType" } }),
    ))
}

/// DELETE /api/resource/:doctype/:name — delete a document
pub async fn delete_doc(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path((doctype, name)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    controller::delete(&ctx, &meta, &name, state.hook_runner.as_ref())
        .await
        .map_err(|e| error_response(&e))?;

    state
        .realtime
        .publish_doc_update(&doctype, &name, "deleted");

    Ok(Json(json!({ "message": "ok" })))
}

/// POST /api/resource/:doctype/:name/submit — submit a document
pub async fn submit_doc(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path((doctype, name)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    let doc = controller::submit(&ctx, &meta, &name, state.hook_runner.as_ref())
        .await
        .map_err(|e| error_response(&e))?;

    state
        .realtime
        .publish_doc_update(&doctype, &name, "submitted");

    Ok(Json(json!({ "data": doc })))
}

/// POST /api/resource/:doctype/:name/cancel — cancel a submitted document
pub async fn cancel_doc(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path((doctype, name)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    let doc = controller::cancel(&ctx, &meta, &name, state.hook_runner.as_ref())
        .await
        .map_err(|e| error_response(&e))?;

    state
        .realtime
        .publish_doc_update(&doctype, &name, "cancelled");

    Ok(Json(json!({ "data": doc })))
}

/// POST /api/resource/:doctype/export-fixtures — export records as fixture JSON
#[derive(Debug, Deserialize)]
pub struct ExportFixtureParams {
    pub filters: Option<String>,
    pub app: Option<String>,
}

pub async fn export_fixtures(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(doctype): Path<String>,
    Query(params): Query<ExportFixtureParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !super::doctype::is_developer_mode() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Export is only available in developer mode."})),
        ));
    }

    let meta = state
        .registry
        .get_meta(&doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    // Parse filters
    let filters: Option<Value> = params
        .filters
        .as_deref()
        .and_then(|f| serde_json::from_str(f).ok());

    // Fetch records
    let docs = controller::get_list(
        &ctx,
        &meta,
        filters.as_ref(),
        None,
        None,
        Some(10000),
        None,
        None,
    )
    .await
    .map_err(|e| error_response(&e))?;

    if docs.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "No records match the filters"})),
        ));
    }

    // Find the app directory
    let slug = doctype.to_lowercase().replace(' ', "_");
    let apps_dir = super::doctype::find_apps_dir();

    let app_name = if let Some(app) = params.app {
        app
    } else {
        super::doctype::find_app_for_doctype(&apps_dir, &slug)
            .or_else(|| {
                let module_slug = meta.module.to_lowercase().replace(' ', "_");
                if apps_dir.join(&module_slug).is_dir() {
                    Some(module_slug)
                } else {
                    None
                }
            })
            .or_else(|| super::doctype::find_single_app(&apps_dir))
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Could not determine target app. Specify ?app=my_app"})),
                )
            })?
    };

    let fixtures_dir = apps_dir.join(&app_name).join("fixtures");
    std::fs::create_dir_all(&fixtures_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create fixtures dir: {}", e)})),
        )
    })?;

    let file_path = fixtures_dir.join(format!("{}.json", slug));
    let pretty = serde_json::to_string_pretty(&docs).unwrap_or_default();
    std::fs::write(&file_path, &pretty).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to write file: {}", e)})),
        )
    })?;

    tracing::info!(
        "Exported {} {} fixture(s) to {:?}",
        docs.len(),
        doctype,
        file_path
    );

    Ok(Json(json!({
        "message": format!("Exported {} record(s) to fixtures/{}.json", docs.len(), slug),
        "count": docs.len(),
        "app": app_name,
        "path": file_path.to_string_lossy(),
    })))
}

/// Map LoomError to HTTP status + JSON body.
fn error_response(e: &loom_core::LoomError) -> (StatusCode, Json<Value>) {
    let status = match e {
        loom_core::LoomError::NotFound { .. } => StatusCode::NOT_FOUND,
        loom_core::LoomError::Validation(_) => StatusCode::BAD_REQUEST,
        loom_core::LoomError::DuplicateEntry(_) => StatusCode::CONFLICT,
        loom_core::LoomError::PermissionDenied(_) => StatusCode::FORBIDDEN,
        loom_core::LoomError::Script(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({"error": e.to_string(), "error_type": e.error_type()})),
    )
}
