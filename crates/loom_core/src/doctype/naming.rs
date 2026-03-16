use sqlx::PgPool;

use crate::error::{LoomError, LoomResult};
use super::meta::{Meta, NamingRule};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generate an `id` (primary key) for a new document based on the DocType's naming rule.
pub fn generate_name(
    meta: &Meta,
    doc: &serde_json::Value,
) -> LoomResult<String> {
    match &meta.naming_rule {
        NamingRule::Autoincrement => {
            // Autoincrement is handled at the DB level; return a placeholder.
            // The actual insert will use a DB sequence.
            Ok(String::new())
        }
        NamingRule::Hash => {
            Ok(Uuid::new_v4().to_string().replace('-', "")[..10].to_string())
        }
        NamingRule::ByFieldname => {
            let field = meta.autoname.as_deref().unwrap_or("name");
            // Strip "by_fieldname(" prefix and ")" suffix if present
            let field = field
                .strip_prefix("by_fieldname(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(field);

            doc.get(field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    LoomError::Validation(format!(
                        "Field '{}' is required for naming in DocType '{}'",
                        field, meta.name
                    ))
                })
        }
        NamingRule::Series => {
            // Series naming (e.g., "HR-LEAVE-.YYYY.-.#####") is handled
            // by the naming series counter in the database.
            // Return the pattern; actual resolution happens at insert time.
            let pattern = meta
                .autoname
                .as_deref()
                .ok_or_else(|| {
                    LoomError::Validation(format!(
                        "DocType '{}' uses series naming but has no autoname pattern",
                        meta.name
                    ))
                })?;
            Ok(pattern.to_string())
        }
        NamingRule::Prompt => {
            // Name must be provided by the user in the document.
            doc.get("id")
                .or_else(|| doc.get("__newname"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    LoomError::Validation(format!(
                        "DocType '{}' requires a name to be provided",
                        meta.name
                    ))
                })
        }
        NamingRule::Expression => {
            let expr = meta
                .autoname
                .as_deref()
                .ok_or_else(|| {
                    LoomError::Validation(format!(
                        "DocType '{}' uses expression naming but has no autoname expression",
                        meta.name
                    ))
                })?;
            resolve_expression(expr, doc)
        }
    }
}

/// Resolve a naming expression like "HR-LEAVE-.YYYY.-.#####" using document fields.
fn resolve_expression(expr: &str, doc: &serde_json::Value) -> LoomResult<String> {
    let now = chrono::Utc::now();
    let mut result = expr.to_string();

    // Replace date placeholders
    result = result.replace(".YYYY.", &now.format("%Y").to_string());
    result = result.replace(".YY.", &now.format("%y").to_string());
    result = result.replace(".MM.", &now.format("%m").to_string());
    result = result.replace(".DD.", &now.format("%d").to_string());

    // Replace field references: {field_name}
    if let Some(obj) = doc.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{}}}", key);
            if result.contains(&placeholder) {
                let val_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => value.to_string(),
                };
                result = result.replace(&placeholder, &val_str);
            }
        }
    }

    // The ##### sequence is a counter — leave it for the DB layer to resolve
    Ok(result)
}

/// Generate a hash-based name using SHA-256.
pub fn hash_name(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..10].to_string()
}

/// Full name resolution: generates the name and resolves series counters from the DB.
/// This is the function the controller should call.
pub async fn resolve_name(
    meta: &Meta,
    doc: &serde_json::Value,
    pool: &PgPool,
) -> LoomResult<String> {
    // If doc already has an id set (and naming isn't autoincrement), use it
    if meta.naming_rule != NamingRule::Autoincrement {
        if let Some(name) = doc.get("id").and_then(|v| v.as_str()) {
            if !name.is_empty() {
                return Ok(name.to_string());
            }
        }
    }

    let name = generate_name(meta, doc)?;

    // If the name contains ##### sequences, resolve them via the series counter
    if name.contains('#') {
        resolve_series(pool, &name).await
    } else if name.is_empty() && meta.naming_rule == NamingRule::Autoincrement {
        // For autoincrement, use a DB sequence-based approach
        resolve_autoincrement(pool, &meta.table_name()).await
    } else {
        Ok(name)
    }
}

/// Atomically increment a counter in the `__naming_series` table and return the
/// resolved name with the ##### pattern replaced by the counter value.
async fn resolve_series(pool: &PgPool, pattern: &str) -> LoomResult<String> {
    // Count the number of # characters to determine padding width
    let hash_count = pattern.chars().filter(|c| *c == '#').count();
    if hash_count == 0 {
        return Ok(pattern.to_string());
    }

    // The prefix is everything before the first #
    let prefix = &pattern[..pattern.find('#').unwrap()];

    // Atomically increment the counter
    let current: i64 = sqlx::query_scalar(
        "INSERT INTO \"__naming_series\" (name, current) VALUES ($1, 1) \
         ON CONFLICT (name) DO UPDATE SET current = \"__naming_series\".current + 1 \
         RETURNING current",
    )
    .bind(prefix)
    .fetch_one(pool)
    .await?;

    // Replace the ##### sequence with the zero-padded counter
    let hash_str = "#".repeat(hash_count);
    let counter_str = format!("{:0>width$}", current, width = hash_count);
    let result = pattern.replace(&hash_str, &counter_str);

    Ok(result)
}

/// Generate an autoincrement name using a DB-level approach.
async fn resolve_autoincrement(pool: &PgPool, table_name: &str) -> LoomResult<String> {
    let current: i64 = sqlx::query_scalar(
        "INSERT INTO \"__naming_series\" (name, current) VALUES ($1, 1) \
         ON CONFLICT (name) DO UPDATE SET current = \"__naming_series\".current + 1 \
         RETURNING current",
    )
    .bind(table_name)
    .fetch_one(pool)
    .await?;

    Ok(current.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_hash_naming() {
        let meta = Meta {
            name: "Test".to_string(),
            module: "Core".to_string(),
            naming_rule: NamingRule::Hash,
            autoname: None,
            is_submittable: false,
            is_child_table: false,
            is_single: false,
            is_virtual: false,
            is_tree: false,
            title_field: None,
            search_fields: vec![],
            sort_field: None,
            sort_order: None,
            fields: vec![],
            permissions: vec![],
            workflow: None,
        };
        let doc = json!({});
        let name = generate_name(&meta, &doc).unwrap();
        assert_eq!(name.len(), 10);
    }

    #[test]
    fn test_prompt_naming() {
        let meta = Meta {
            name: "Test".to_string(),
            module: "Core".to_string(),
            naming_rule: NamingRule::Prompt,
            autoname: None,
            is_submittable: false,
            is_child_table: false,
            is_single: false,
            is_virtual: false,
            is_tree: false,
            title_field: None,
            search_fields: vec![],
            sort_field: None,
            sort_order: None,
            fields: vec![],
            permissions: vec![],
            workflow: None,
        };
        let doc = json!({"id": "my-custom-name"});
        let name = generate_name(&meta, &doc).unwrap();
        assert_eq!(name, "my-custom-name");
    }
}
