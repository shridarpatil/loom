use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::server::AppState;

/// GET /api/config/theme — public, returns theme JSON or defaults
pub async fn get_theme(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let result = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT value FROM \"__site_config\" WHERE key = 'theme'"
    )
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(value)) => Json(serde_json::json!({ "data": value })).into_response(),
        Ok(None) => Json(serde_json::json!({
            "data": {
                "brand_name": "Loom",
                "logo_url": "",
                "primary_color": "#4f46e5",
                "font_family": "Inter",
                "radius": "0.375rem"
            }
        })).into_response(),
        Err(e) => {
            tracing::error!("Failed to read theme: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to read theme"
            }))).into_response()
        }
    }
}

/// PUT /api/config/theme — admin only, saves theme
pub async fn save_theme(
    State(state): State<AppState>,
    Extension(ctx): Extension<loom_core::context::RequestContext>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Admin only
    if !ctx.is_administrator() && !ctx.has_role("System Manager") {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({
            "error": "Only administrators can update theme"
        }))).into_response();
    }

    let result = sqlx::query(
        "INSERT INTO \"__site_config\" (key, value, modified)
         VALUES ('theme', $1, NOW())
         ON CONFLICT (key) DO UPDATE SET value = $1, modified = NOW()"
    )
    .bind(&body)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({ "message": "Theme saved" })).into_response(),
        Err(e) => {
            tracing::error!("Failed to save theme: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to save theme"
            }))).into_response()
        }
    }
}
