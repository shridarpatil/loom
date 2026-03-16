use axum::{
    extract::{Extension, Multipart},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use loom_core::context::RequestContext;

/// POST /api/upload — handle file uploads
/// Stores files to sites/uploads/ and returns the URL.
pub async fn upload_file(
    Extension(ctx): Extension<RequestContext>,
    mut multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let upload_dir = find_upload_dir();
    std::fs::create_dir_all(&upload_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to create upload dir: {}", e)})),
        )
    })?;

    let mut uploaded = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Multipart error: {}", e)})),
            )
        })?
    {
        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("upload_{}", chrono::Utc::now().timestamp_millis()));

        // Sanitize filename
        let safe_name = sanitize_filename(&file_name);
        let unique_name = format!(
            "{}_{}",
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            safe_name
        );

        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Failed to read file: {}", e)})),
            )
        })?;

        let file_path = upload_dir.join(&unique_name);
        std::fs::write(&file_path, &data).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to write file: {}", e)})),
            )
        })?;

        let url = format!("/uploads/{}", unique_name);
        tracing::info!("File uploaded by {}: {} ({} bytes)", ctx.user, url, data.len());

        uploaded.push(json!({
            "file_name": safe_name,
            "file_url": url,
            "file_size": data.len(),
        }));
    }

    if uploaded.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No file provided"})),
        ));
    }

    Ok(Json(json!({
        "message": "File uploaded",
        "data": if uploaded.len() == 1 { uploaded[0].clone() } else { json!(uploaded) }
    })))
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn find_upload_dir() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        let sites = dir.join("sites");
        if sites.is_dir() {
            return sites.join("uploads");
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir()
        .unwrap_or_default()
        .join("sites")
        .join("uploads")
}
