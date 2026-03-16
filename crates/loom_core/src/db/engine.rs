use serde_json::Value;
use sqlx::PgPool;

use crate::doctype::meta::doctype_table_name;
use crate::error::LoomResult;

/// High-level database operations that wrap raw SQL with ergonomic APIs.
/// These correspond to the `loom.db.*` methods available in Rhai scripts.

/// Get a single field value from a document matching filters.
pub async fn get_value(
    pool: &PgPool,
    doctype: &str,
    filters: &Value,
    fieldname: &str,
) -> LoomResult<Option<Value>> {
    let table = doctype_table_name(doctype);
    let mut sql = format!("SELECT \"{}\" FROM \"{}\"", fieldname, table);
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(obj) = filters.as_object() {
        let mut where_clauses = Vec::new();
        for (key, value) in obj {
            bind_values.push(match value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            });
            where_clauses.push(format!("\"{}\" = ${}", key, bind_values.len()));
        }
        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }
    } else if let Some(name) = filters.as_str() {
        bind_values.push(name.to_string());
        sql.push_str(&format!(" WHERE id = ${}", bind_values.len()));
    }

    sql.push_str(" LIMIT 1");

    let mut query = sqlx::query_scalar::<_, Value>(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }

    let result = query.fetch_optional(pool).await?;
    Ok(result)
}

/// Set a field value on a document.
pub async fn set_value(
    pool: &PgPool,
    doctype: &str,
    name: &str,
    field: &str,
    value: &Value,
) -> LoomResult<()> {
    let table = doctype_table_name(doctype);
    let sql = format!(
        "UPDATE \"{}\" SET \"{}\" = $1, modified = NOW() WHERE id = $2",
        table, field
    );

    let val_str = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };

    sqlx::query(&sql)
        .bind(&val_str)
        .bind(name)
        .execute(pool)
        .await?;

    Ok(())
}

/// Check if a document exists matching filters.
pub async fn exists(pool: &PgPool, doctype: &str, filters: &Value) -> LoomResult<bool> {
    let table = doctype_table_name(doctype);
    let mut sql = format!("SELECT EXISTS (SELECT 1 FROM \"{}\"", table);
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(obj) = filters.as_object() {
        let mut where_clauses = Vec::new();
        for (key, value) in obj {
            bind_values.push(match value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            });
            where_clauses.push(format!("\"{}\" = ${}", key, bind_values.len()));
        }
        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }
    }

    sql.push(')');

    let mut query = sqlx::query_scalar::<_, bool>(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }

    let result = query.fetch_one(pool).await?;
    Ok(result)
}

/// Get count of documents matching filters.
pub async fn count(pool: &PgPool, doctype: &str, filters: Option<&Value>) -> LoomResult<i64> {
    let table = doctype_table_name(doctype);
    let mut sql = format!("SELECT COUNT(*) FROM \"{}\"", table);
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(filters) = filters {
        if let Some(obj) = filters.as_object() {
            let mut where_clauses = Vec::new();
            for (key, value) in obj {
                bind_values.push(match value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
                where_clauses.push(format!("\"{}\" = ${}", key, bind_values.len()));
            }
            if !where_clauses.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&where_clauses.join(" AND "));
            }
        }
    }

    let mut query = sqlx::query_scalar::<_, i64>(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }

    let result = query.fetch_one(pool).await?;
    Ok(result)
}
