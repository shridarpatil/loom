use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::Value;
use sqlx::PgPool;

use loom_core::context::RequestContext;

use crate::cache::AppCache;
use crate::routes::auth::extract_sid;
use crate::server::AppState;

/// Inject a `RequestContext` into the request extensions based on auth.
/// Uses session cache to avoid DB queries on every request.
pub async fn inject_context(state: AppState, mut request: Request, next: Next) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());

    let cookie_header = request
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());

    let resolved = resolve_user(
        &state.pool,
        &state.cache,
        auth_header.as_deref(),
        cookie_header.as_deref(),
    )
    .await;

    match resolved {
        Some((user, roles)) => {
            let ctx = RequestContext::new(
                user,
                roles,
                "default".to_string(),
                state.pool.clone(),
                state.registry.clone(),
            );
            request.extensions_mut().insert(ctx);
            next.run(request).await
        }
        None => {
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "Not authenticated", "error_type": "AuthenticationError"})),
            )
                .into_response()
        }
    }
}

/// Resolve user identity from session cookie or auth header.
/// Checks cache first, falls back to DB.
async fn resolve_user(
    pool: &PgPool,
    cache: &AppCache,
    auth_header: Option<&str>,
    cookie_header: Option<&str>,
) -> Option<(String, Vec<String>)> {
    // 1. Check session cookie (with cache)
    if let Some(cookies) = cookie_header {
        if let Some(sid) = extract_sid(cookies) {
            // Try cache first
            if let Some(cached) = cache.sessions.get(&sid).await {
                return Some(cached);
            }
            // Cache miss — hit DB
            if let Some((user, roles)) = validate_session(pool, &sid).await {
                cache.sessions.set(sid, (user.clone(), roles.clone())).await;
                return Some((user, roles));
            }
        }
    }

    // 2. Check API key auth header (no caching — rare path)
    if let Some(header) = auth_header {
        if header.starts_with("token ") {
            let token = &header[6..];
            if let Some((api_key, api_secret)) = token.split_once(':') {
                if let Some(user_name) = validate_api_key(pool, api_key, api_secret).await {
                    let roles = get_user_roles(pool, &user_name).await;
                    return Some((user_name, roles));
                }
            }
        }
    }

    None
}

/// Validate a session ID and return (user_email, roles) if valid.
async fn validate_session(pool: &PgPool, sid: &str) -> Option<(String, Vec<String>)> {
    let user_email: Option<String> = sqlx::query_scalar(
        "SELECT user_email FROM \"__session\" WHERE sid = $1 AND expires > NOW()",
    )
    .bind(sid)
    .fetch_optional(pool)
    .await
    .ok()?;

    let email = user_email?;

    // Verify user is enabled — check tabUser first, fallback to __user
    let enabled = check_user_enabled(pool, &email).await;
    if !enabled {
        return None;
    }

    let roles = get_user_roles(pool, &email).await;
    Some((email, roles))
}

/// Check if a user is enabled. Tries User DocType table first, falls back to __user.
async fn check_user_enabled(pool: &PgPool, email: &str) -> bool {
    let user_table = loom_core::doctype::doctype_table_name("User");
    let sql = format!("SELECT enabled::TEXT FROM \"{}\" WHERE id = $1", user_table);
    let result: Option<String> = sqlx::query_scalar(&sql)
        .bind(email)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    if let Some(val) = result {
        return val == "true" || val == "t" || val == "1" || val == "yes";
    }

    // Fallback to __user
    let result: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM \"__user\" WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    result.unwrap_or(false)
}

/// Get roles for a user. Tries User DocType table first, falls back to __user (legacy).
async fn get_user_roles(pool: &PgPool, email: &str) -> Vec<String> {
    let user_table = loom_core::doctype::doctype_table_name("User");
    let sql = format!(
        "SELECT roles_json FROM \"{}\" WHERE id = $1 AND enabled = true",
        user_table
    );
    let roles_json: Option<Value> = sqlx::query_scalar(&sql)
        .bind(email)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    if let Some(val) = roles_json {
        return parse_roles_json(&val);
    }

    // Fallback to __user (legacy)
    let roles_json: Option<Value> =
        sqlx::query_scalar("SELECT roles FROM \"__user\" WHERE email = $1 AND enabled = TRUE")
            .bind(email)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    parse_roles_json(&roles_json.unwrap_or(Value::Null))
}

fn parse_roles_json(val: &Value) -> Vec<String> {
    match val {
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Value::String(s) => {
            // Might be a JSON string stored as text
            serde_json::from_str::<Vec<String>>(s).unwrap_or_else(|_| vec!["All".to_string()])
        }
        _ => vec!["All".to_string()],
    }
}

/// Validate an API key/secret pair against the __user_api_key table.
async fn validate_api_key(pool: &PgPool, api_key: &str, api_secret: &str) -> Option<String> {
    sqlx::query_scalar(
        "SELECT user_name FROM \"__user_api_key\" WHERE api_key = $1 AND api_secret = $2",
    )
    .bind(api_key)
    .bind(api_secret)
    .fetch_optional(pool)
    .await
    .ok()?
}
