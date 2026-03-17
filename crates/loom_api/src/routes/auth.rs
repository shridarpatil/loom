use axum::{
    extract::State,
    http::{header::SET_COOKIE, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use loom_core::db::migrate::{hash_password, verify_password};

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// POST /api/auth/login — authenticate and create session
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    // Look up user — try tabUser (DocType) first, fallback to __user (legacy)
    let (email, password_hash, full_name, enabled, roles) =
        match lookup_user(&state.pool, &body.email).await {
            Some(u) => u,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Invalid email or password"})),
                ))
            }
        };

    if !enabled {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Account is disabled"})),
        ));
    }

    if !verify_password(&body.password, &password_hash) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid email or password"})),
        ));
    }

    // Create session
    let sid = Uuid::new_v4().to_string();
    let expires_hours = 24 * 7; // 7 days

    sqlx::query(
        "INSERT INTO \"__session\" (sid, user_email, expires) VALUES ($1, $2, NOW() + $3 * INTERVAL '1 hour')",
    )
    .bind(&sid)
    .bind(&email)
    .bind(expires_hours as i32)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    // Set cookie
    let cookie = format!(
        "sid={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        sid,
        expires_hours * 3600,
    );

    let body = Json(json!({
        "user": email,
        "full_name": full_name,
        "roles": roles,
    }));

    Ok(([(SET_COOKIE, cookie)], body))
}

/// POST /api/auth/logout — destroy session
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    // Extract sid from cookie
    if let Some(cookie_header) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        if let Some(sid) = extract_sid(cookie_header) {
            sqlx::query("DELETE FROM \"__session\" WHERE sid = $1")
                .bind(&sid)
                .execute(&state.pool)
                .await
                .ok();
            // Invalidate session cache
            state.cache.invalidate_session(&sid).await;
        }
    }

    // Clear cookie
    let cookie = "sid=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    Ok((
        [(SET_COOKIE, cookie.to_string())],
        Json(json!({"message": "Logged out"})),
    ))
}

/// POST /api/auth/signup — create a new user account
#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub full_name: Option<String>,
}

pub async fn signup(
    State(state): State<AppState>,
    Json(body): Json<SignupRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    if body.email.is_empty() || body.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Email and password are required"})),
        ));
    }

    let user_table = loom_core::doctype::doctype_table_name("User");

    // Check if user exists (check both User table and __user)
    let check_sql = format!(
        "SELECT EXISTS (SELECT 1 FROM \"{}\" WHERE id = $1 OR email = $1)",
        user_table
    );
    let exists_tab: bool = sqlx::query_scalar(&check_sql)
        .bind(&body.email)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(false); // Gracefully handle if User table doesn't exist

    let exists_legacy: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM \"__user\" WHERE email = $1)")
            .bind(&body.email)
            .fetch_one(&state.pool)
            .await
            .unwrap_or(false);

    if exists_tab || exists_legacy {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"error": "User already exists"})),
        ));
    }

    let password_hash = hash_password(&body.password);
    let full_name = body.full_name.as_deref().unwrap_or("");
    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();

    // Insert into User DocType table
    let insert_sql = format!(
        "INSERT INTO \"{}\" (id, email, full_name, password_hash, enabled, roles_json, owner, creation, modified, modified_by, docstatus) \
         VALUES ($1, $2, $3, $4, 'true', '[\"All\"]', $1, $5::TIMESTAMP, $5::TIMESTAMP, $1, 0)",
        user_table
    );
    sqlx::query(&insert_sql)
        .bind(&body.email)
        .bind(&body.email)
        .bind(full_name)
        .bind(&password_hash)
        .bind(&now)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(json!({"message": "User created", "email": body.email})),
    ))
}

/// Look up a user for login. Tries User DocType table first, falls back to __user.
/// Returns (email, password_hash, full_name, enabled, roles_json).
async fn lookup_user(
    pool: &sqlx::PgPool,
    email: &str,
) -> Option<(String, String, String, bool, Value)> {
    let user_table = loom_core::doctype::doctype_table_name("User");

    // Try User DocType table (gracefully handle if table doesn't exist yet)
    let select_sql = format!(
        "SELECT id, email, full_name, password_hash, roles_json FROM \"{}\" WHERE id = $1 OR email = $1",
        user_table
    );
    let row: Option<(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<Value>,
    )> = sqlx::query_as(&select_sql)
        .bind(email)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

    if let Some((id, _email_col, full_name, password_hash, roles_json)) = row {
        let password_hash = password_hash.unwrap_or_default();
        let full_name = full_name.unwrap_or_default();
        let enabled_sql = format!("SELECT enabled::TEXT FROM \"{}\" WHERE id = $1", user_table);
        let enabled_val: Option<String> = sqlx::query_scalar(&enabled_sql)
            .bind(&id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
        let enabled = matches!(
            enabled_val.as_deref(),
            Some("true") | Some("t") | Some("1") | Some("yes")
        );

        let roles = roles_json.unwrap_or(json!(["All"]));
        // roles_json might be stored as a JSON string inside a text column
        let roles = match &roles {
            Value::String(s) => serde_json::from_str(s).unwrap_or(json!(["All"])),
            other => other.clone(),
        };
        return Some((id, password_hash, full_name, enabled, roles));
    }

    // Fallback to __user (legacy)
    let row: Option<(String, String, String, bool)> = sqlx::query_as(
        "SELECT email, password_hash, full_name, enabled FROM \"__user\" WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .ok()?;

    let (email, password_hash, full_name, enabled) = row?;

    let roles: Value = sqlx::query_scalar("SELECT roles FROM \"__user\" WHERE email = $1")
        .bind(&email)
        .fetch_one(pool)
        .await
        .ok()
        .unwrap_or(json!(["All"]));

    Some((email, password_hash, full_name, enabled, roles))
}

/// Extract session ID from cookie header string
pub fn extract_sid(cookie_header: &str) -> Option<String> {
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("sid=") {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}
