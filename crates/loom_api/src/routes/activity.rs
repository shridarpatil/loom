use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use loom_core::context::RequestContext;
use loom_core::db::activity;
use loom_core::perms::{check_permission, PermType};

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct ActivityParams {
    pub limit: Option<i64>,
}

/// GET /api/activity/:doctype/:name — get activity timeline for a document
pub async fn get_activity(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path((doctype, name)): Path<(String, String)>,
    Query(params): Query<ActivityParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check read permission on the doctype
    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    if !ctx.ignore_permissions() {
        check_permission(&meta, None, PermType::Read, &ctx.user, &ctx.roles)
            .map_err(|e| (StatusCode::FORBIDDEN, Json(json!({"error": e.to_string()}))))?;
    }

    let limit = params.limit.unwrap_or(50);
    let timeline = activity::get_activity(ctx.pool(), &doctype, &name, limit)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    Ok(Json(json!({ "data": timeline })))
}

#[derive(Debug, Deserialize)]
pub struct CommentBody {
    pub content: String,
}

/// POST /api/activity/:doctype/:name/comment — add a comment
pub async fn add_comment(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path((doctype, name)): Path<(String, String)>,
    Json(body): Json<CommentBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Check read permission on the doctype
    let meta = state
        .cache
        .get_effective_meta(&state.registry, &state.pool, &doctype)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))))?;

    if !ctx.ignore_permissions() {
        check_permission(&meta, None, PermType::Read, &ctx.user, &ctx.roles)
            .map_err(|e| (StatusCode::FORBIDDEN, Json(json!({"error": e.to_string()}))))?;
    }

    activity::add_comment(ctx.pool(), &doctype, &name, &ctx.user, &body.content)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    Ok(Json(json!({ "message": "Comment added" })))
}
