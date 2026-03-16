use serde_json::{json, Value};
use sqlx::PgPool;

use crate::error::{LoomError, LoomResult};
use super::meta::{FieldType, Meta};

/// Set parent metadata on child rows and ensure idx ordering.
pub fn prepare_child_rows(
    parent_name: &str,
    parent_doctype: &str,
    fieldname: &str,
    rows: &mut Vec<Value>,
) -> LoomResult<()> {
    for (i, row) in rows.iter_mut().enumerate() {
        let obj = row
            .as_object_mut()
            .ok_or_else(|| LoomError::Validation("Child row must be a JSON object".into()))?;

        obj.insert("parent".to_string(), Value::String(parent_name.to_string()));
        obj.insert(
            "parentfield".to_string(),
            Value::String(fieldname.to_string()),
        );
        obj.insert(
            "parenttype".to_string(),
            Value::String(parent_doctype.to_string()),
        );
        obj.insert("idx".to_string(), Value::Number((i as i64 + 1).into()));
    }
    Ok(())
}

/// Extract child table rows from a parent document for a given fieldname.
pub fn extract_child_rows(doc: &Value, fieldname: &str) -> Vec<Value> {
    doc.get(fieldname)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Save child table rows for a parent document during insert.
/// Recurses into grandchildren if the child DocType also has Table fields.
pub async fn insert_children(
    pool: &PgPool,
    parent_meta: &Meta,
    parent_name: &str,
    doc: &mut Value,
    user: &str,
    registry: &crate::doctype::DocTypeRegistry,
) -> LoomResult<()> {
    for field in parent_meta.fields.iter() {
        if field.fieldtype != FieldType::Table {
            continue;
        }
        let child_doctype = match &field.options {
            Some(dt) if !dt.is_empty() => dt,
            _ => continue,
        };

        let mut rows = extract_child_rows(doc, &field.fieldname);
        if rows.is_empty() {
            continue;
        }

        let child_meta = match registry.get_meta(child_doctype).await {
            Ok(m) => m,
            Err(_) => continue,
        };

        prepare_child_rows(parent_name, &parent_meta.name, &field.fieldname, &mut rows)?;

        let table = child_meta.table_name();
        for row in &mut rows {
            set_standard_fields(row, user);
            generate_child_id(row, parent_name, &field.fieldname);
            insert_child_row(pool, &table, &child_meta, row).await?;

            // Recurse: insert grandchildren if this child has Table fields
            let child_id = row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if !child_id.is_empty() && has_table_fields(&child_meta) {
                // Use Box::pin for recursive async
                Box::pin(insert_children(pool, &child_meta, &child_id, row, user, registry)).await?;
            }
        }

        if let Some(obj) = doc.as_object_mut() {
            obj.insert(field.fieldname.clone(), json!(rows));
        }
    }
    Ok(())
}

/// Update child table rows for a parent document.
/// Deletes existing children (and their grandchildren), then re-inserts.
pub async fn update_children(
    pool: &PgPool,
    parent_meta: &Meta,
    parent_name: &str,
    doc: &mut Value,
    user: &str,
    registry: &crate::doctype::DocTypeRegistry,
) -> LoomResult<()> {
    for field in parent_meta.fields.iter() {
        if field.fieldtype != FieldType::Table {
            continue;
        }
        let child_doctype = match &field.options {
            Some(dt) if !dt.is_empty() => dt,
            _ => continue,
        };

        if !doc.as_object().map_or(false, |o| o.contains_key(&field.fieldname)) {
            continue;
        }

        let mut rows = extract_child_rows(doc, &field.fieldname);

        let child_meta = match registry.get_meta(child_doctype).await {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Delete existing children (recurse into grandchildren first)
        Box::pin(delete_children_for_field(pool, &child_meta, parent_name, &field.fieldname, registry)).await?;

        if rows.is_empty() {
            continue;
        }

        prepare_child_rows(parent_name, &parent_meta.name, &field.fieldname, &mut rows)?;

        let table = child_meta.table_name();
        for row in &mut rows {
            set_standard_fields(row, user);
            generate_child_id(row, parent_name, &field.fieldname);
            insert_child_row(pool, &table, &child_meta, row).await?;

            // Recurse: insert grandchildren
            let child_id = row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if !child_id.is_empty() && has_table_fields(&child_meta) {
                Box::pin(insert_children(pool, &child_meta, &child_id, row, user, registry)).await?;
            }
        }

        if let Some(obj) = doc.as_object_mut() {
            obj.insert(field.fieldname.clone(), json!(rows));
        }
    }
    Ok(())
}

/// Delete all child rows when a parent document is deleted.
/// Recurses into grandchildren.
pub async fn delete_children(
    pool: &PgPool,
    parent_meta: &Meta,
    parent_name: &str,
    registry: &crate::doctype::DocTypeRegistry,
) -> LoomResult<()> {
    for field in parent_meta.fields.iter() {
        if field.fieldtype != FieldType::Table {
            continue;
        }
        let child_doctype = match &field.options {
            Some(dt) if !dt.is_empty() => dt,
            _ => continue,
        };

        let child_meta = match registry.get_meta(child_doctype).await {
            Ok(m) => m,
            Err(_) => continue,
        };

        // If the child itself has Table fields, find child IDs and recurse first
        if has_table_fields(&child_meta) {
            let table = child_meta.table_name();
            let ids: Vec<String> = sqlx::query_scalar(&format!(
                "SELECT id FROM \"{}\" WHERE parent = $1",
                table
            ))
            .bind(parent_name)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            for child_id in &ids {
                Box::pin(delete_children(pool, &child_meta, child_id, registry)).await?;
            }
        }

        // Now delete the direct children
        let table = child_meta.table_name();
        let sql = format!("DELETE FROM \"{}\" WHERE parent = $1", table);
        sqlx::query(&sql)
            .bind(parent_name)
            .execute(pool)
            .await
            .ok();
    }
    Ok(())
}

/// Delete children for a specific parent + fieldname, recursing into grandchildren.
async fn delete_children_for_field(
    pool: &PgPool,
    child_meta: &Meta,
    parent_name: &str,
    fieldname: &str,
    registry: &crate::doctype::DocTypeRegistry,
) -> LoomResult<()> {
    let table = child_meta.table_name();

    // If the child has Table fields, recurse into grandchildren first
    if has_table_fields(child_meta) {
        let ids: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT id FROM \"{}\" WHERE parent = $1 AND parentfield = $2",
            table
        ))
        .bind(parent_name)
        .bind(fieldname)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for child_id in &ids {
            Box::pin(delete_children(pool, child_meta, child_id, registry)).await?;
        }
    }

    let sql = format!(
        "DELETE FROM \"{}\" WHERE parent = $1 AND parentfield = $2",
        table
    );
    sqlx::query(&sql)
        .bind(parent_name)
        .bind(fieldname)
        .execute(pool)
        .await
        .ok();

    Ok(())
}

// --- Helpers ---

fn has_table_fields(meta: &Meta) -> bool {
    meta.fields.iter().any(|f| f.fieldtype == FieldType::Table)
}

use super::meta::set_standard_fields_on_insert as set_standard_fields;

fn generate_child_id(row: &mut Value, _parent_name: &str, _fieldname: &str) {
    if let Some(obj) = row.as_object_mut() {
        if obj.get("id").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            let id = uuid::Uuid::new_v4().to_string().replace('-', "")[..10].to_string();
            obj.insert("id".to_string(), json!(id));
        }
    }
}

/// Insert a single child row into its table.
async fn insert_child_row(
    pool: &PgPool,
    table: &str,
    child_meta: &Meta,
    row: &Value,
) -> LoomResult<()> {
    let obj = row
        .as_object()
        .ok_or_else(|| LoomError::Validation("Child row must be a JSON object".into()))?;

    let mut columns = Vec::new();
    let mut values: Vec<String> = Vec::new();

    for (key, value) in obj {
        // Skip Table fields — they're handled by recursion, not stored as columns
        if let Some(field_meta) = child_meta.get_field(key) {
            if field_meta.fieldtype == FieldType::Table {
                continue;
            }
        }

        let is_standard = super::meta::STANDARD_FIELDS
            .iter()
            .any(|(name, _)| *name == key.as_str());
        let is_meta_field = child_meta
            .get_field(key)
            .is_some_and(|f| f.fieldtype.has_column());

        if is_standard || is_meta_field {
            columns.push(format!("\"{}\"", key));
            values.push(match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "NULL".to_string(),
                _ => value.to_string(),
            });
        }
    }

    if columns.is_empty() {
        return Ok(());
    }

    let sql = format!(
        "INSERT INTO \"{}\" ({}) VALUES ({})",
        table,
        columns.join(", "),
        values
            .iter()
            .map(|v| format!("'{}'", v.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ")
    );

    sqlx::query(&sql)
        .execute(pool)
        .await
        .map_err(|e| LoomError::Internal(format!("Failed to insert child row: {}", e)))?;

    Ok(())
}
