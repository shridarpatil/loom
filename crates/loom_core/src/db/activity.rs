use serde_json::Value;
use sqlx::PgPool;

use crate::error::{LoomError, LoomResult};

/// Log a document activity event.
pub async fn log_activity(
    pool: &PgPool,
    doctype: &str,
    docname: &str,
    action: &str,
    user: &str,
    data: &Value,
) -> LoomResult<()> {
    sqlx::query(
        "INSERT INTO \"__activity\" (doctype, docname, action, user_email, timestamp, data) \
         VALUES ($1, $2, $3, $4, NOW(), $5)",
    )
    .bind(doctype)
    .bind(docname)
    .bind(action)
    .bind(user)
    .bind(data)
    .execute(pool)
    .await
    .map_err(|e| LoomError::Internal(format!("Failed to log activity: {}", e)))?;
    Ok(())
}

/// Get activity timeline for a document, ordered newest first.
pub async fn get_activity(
    pool: &PgPool,
    doctype: &str,
    docname: &str,
    limit: i64,
) -> LoomResult<Vec<Value>> {
    let rows: Vec<(
        i64,
        String,
        String,
        String,
        String,
        Option<chrono::NaiveDateTime>,
        Value,
    )> = sqlx::query_as(
        "SELECT id, doctype, docname, action, user_email, timestamp, data \
             FROM \"__activity\" \
             WHERE doctype = $1 AND docname = $2 \
             ORDER BY timestamp DESC \
             LIMIT $3",
    )
    .bind(doctype)
    .bind(docname)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| LoomError::Internal(format!("Failed to get activity: {}", e)))?;

    let result: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, doctype, docname, action, user_email, timestamp, data)| {
                serde_json::json!({
                    "id": id,
                    "doctype": doctype,
                    "docname": docname,
                    "action": action,
                    "user": user_email,
                    "timestamp": timestamp.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
                    "data": data,
                })
            },
        )
        .collect();

    Ok(result)
}

/// Add a comment to a document's activity timeline.
pub async fn add_comment(
    pool: &PgPool,
    doctype: &str,
    docname: &str,
    user: &str,
    content: &str,
) -> LoomResult<()> {
    log_activity(
        pool,
        doctype,
        docname,
        "Commented",
        user,
        &serde_json::json!({ "comment": content }),
    )
    .await
}
