use serde_json::Value;
use sqlx::PgPool;

use super::meta::{FieldType, Meta, STANDARD_FIELDS};
use crate::error::{LoomError, LoomResult};

/// Insert a new document into the database.
pub async fn insert_doc(
    pool: &PgPool,
    meta: &Meta,
    doc: &mut Value,
    user: &str,
) -> LoomResult<Value> {
    validate_required_fields(meta, doc)?;
    set_standard_fields_on_insert(doc, user);

    let table = meta.table_name();
    let obj = doc
        .as_object()
        .ok_or_else(|| LoomError::Validation("Document must be a JSON object".to_string()))?;

    let mut columns = Vec::new();
    let mut placeholders = Vec::new();
    let mut values: Vec<String> = Vec::new();
    let mut idx = 1;

    for (key, value) in obj {
        // Only insert fields that exist as standard fields or in the meta
        let is_standard = STANDARD_FIELDS.iter().any(|(name, _)| *name == key.as_str());
        let is_meta_field = meta.get_field(key).is_some_and(|f| f.fieldtype.has_column());

        if is_standard || is_meta_field {
            columns.push(format!("\"{}\"", key));
            placeholders.push(format!("${}", idx));
            values.push(value_to_sql_string(value));
            idx += 1;
        }
    }

    let sql = format!(
        "INSERT INTO \"{}\" ({}) VALUES ({}) RETURNING *",
        table,
        columns.join(", "),
        placeholders.join(", ")
    );

    tracing::debug!("INSERT SQL: {}", sql);

    // For now, use a simple text-based query approach.
    // In production, this would use proper parameterized queries with sqlx::query_as.
    let row = sqlx::query_scalar::<_, Value>(&format!(
        "INSERT INTO \"{}\" ({}) VALUES ({}) RETURNING row_to_json(\"{}\".*)",
        table,
        columns.join(", "),
        // Build actual value literals for now — will be replaced with proper bind params
        values
            .iter()
            .map(|v| format!("'{}'", v.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", "),
        table
    ))
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Fetch a single document by name.
pub async fn get_doc(pool: &PgPool, meta: &Meta, name: &str) -> LoomResult<Value> {
    let table = meta.table_name();
    let sql = format!(
        "SELECT row_to_json(t.*) FROM \"{}\" t WHERE t.id = $1",
        table
    );

    let row: Option<Value> = sqlx::query_scalar(&sql)
        .bind(name)
        .fetch_optional(pool)
        .await?;

    row.ok_or_else(|| LoomError::NotFound {
        doctype: meta.name.clone(),
        name: name.to_string(),
    })
}

/// Fetch a list of documents with optional filters, fields, ordering, pagination, and search.
pub async fn get_list(
    pool: &PgPool,
    meta: &Meta,
    filters: Option<&Value>,
    fields: Option<&[&str]>,
    order_by: Option<&str>,
    limit: Option<u32>,
    offset: Option<u32>,
    search_text: Option<&str>,
) -> LoomResult<Vec<Value>> {
    let table = meta.table_name();

    let select_cols = match fields {
        Some(f) => f
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(", "),
        None => "*".to_string(),
    };

    let mut sql = format!(
        "SELECT row_to_json(t) FROM (SELECT {} FROM \"{}\"",
        select_cols, table
    );
    let mut bind_values: Vec<String> = Vec::new();
    let mut where_clauses: Vec<String> = Vec::new();

    if let Some(filters) = filters {
        parse_filters_into(filters, meta, &mut bind_values, &mut where_clauses);
    }

    // Full-text search on name, title_field, search_fields, and text-like fields
    if let Some(term) = search_text {
        if !term.is_empty() {
            let pattern = format!("%{}%", term);
            bind_values.push(pattern);
            let param_idx = bind_values.len();

            let mut search_cols = vec!["id".to_string()];

            // Add title_field
            if let Some(ref tf) = meta.title_field {
                if !search_cols.contains(tf) {
                    search_cols.push(tf.clone());
                }
            }

            // Add search_fields
            for sf in &meta.search_fields {
                if !search_cols.contains(sf) {
                    search_cols.push(sf.clone());
                }
            }

            // If only "id" so far, fall back to all text-compatible data fields
            if search_cols.len() <= 1 {
                for f in meta.data_fields() {
                    if matches!(
                        f.fieldtype,
                        super::meta::FieldType::Data
                            | super::meta::FieldType::Link
                            | super::meta::FieldType::Select
                            | super::meta::FieldType::Text
                            | super::meta::FieldType::SmallText
                            | super::meta::FieldType::LongText
                    ) && !search_cols.contains(&f.fieldname)
                    {
                        search_cols.push(f.fieldname.clone());
                    }
                }
            }

            let search_parts: Vec<String> = search_cols
                .iter()
                .map(|col| format!("\"{}\" ILIKE ${}", col, param_idx))
                .collect();
            where_clauses.push(format!("({})", search_parts.join(" OR ")));
        }
    }

    if !where_clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }

    let order = sanitize_order_by(order_by.unwrap_or("modified DESC"));
    sql.push_str(&format!(" ORDER BY {}", order));

    if let Some(limit) = limit {
        sql.push_str(&format!(" LIMIT {}", limit));
    } else {
        sql.push_str(" LIMIT 20");
    }

    if let Some(offset) = offset {
        sql.push_str(&format!(" OFFSET {}", offset));
    }

    sql.push_str(") t");

    tracing::debug!("LIST SQL: {}", sql);

    // Build the query with binds
    let mut query = sqlx::query_scalar::<_, Value>(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }

    let rows = query.fetch_all(pool).await?;
    Ok(rows)
}

/// Update an existing document.
pub async fn update_doc(
    pool: &PgPool,
    meta: &Meta,
    name: &str,
    doc: &mut Value,
    user: &str,
) -> LoomResult<Value> {
    set_standard_fields_on_update(doc, user);

    let table = meta.table_name();
    let obj = doc
        .as_object()
        .ok_or_else(|| LoomError::Validation("Document must be a JSON object".to_string()))?;

    let mut set_clauses = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();
    let mut idx = 1;

    for (key, value) in obj {
        if key == "id" || key == "creation" || key == "owner" {
            continue; // Don't update immutable fields
        }
        let is_standard = STANDARD_FIELDS.iter().any(|(n, _)| *n == key.as_str());
        let is_meta_field = meta.get_field(key).is_some_and(|f| f.fieldtype.has_column());

        if is_standard || is_meta_field {
            let cast = sql_cast_for_key(key, meta);
            set_clauses.push(format!("\"{}\" = ${}{}", key, idx, cast));
            bind_values.push(value_to_sql_string(value));
            idx += 1;
        }
    }

    if set_clauses.is_empty() {
        return get_doc(pool, meta, name).await;
    }

    let sql = format!(
        "UPDATE \"{}\" SET {} WHERE id = ${} RETURNING row_to_json(\"{}\".*)",
        table,
        set_clauses.join(", "),
        idx,
        table
    );

    let mut query = sqlx::query_scalar::<_, Value>(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }
    query = query.bind(name);

    let row = query.fetch_optional(pool).await?;
    row.ok_or_else(|| LoomError::NotFound {
        doctype: meta.name.clone(),
        name: name.to_string(),
    })
}

/// Delete a document by name.
pub async fn delete_doc(pool: &PgPool, meta: &Meta, name: &str) -> LoomResult<()> {
    let table = meta.table_name();
    let sql = format!("DELETE FROM \"{}\" WHERE id = $1", table);

    let result = sqlx::query(&sql).bind(name).execute(pool).await?;

    if result.rows_affected() == 0 {
        return Err(LoomError::NotFound {
            doctype: meta.name.clone(),
            name: name.to_string(),
        });
    }

    Ok(())
}

/// Get the count of documents matching filters.
pub async fn get_count(
    pool: &PgPool,
    meta: &Meta,
    filters: Option<&Value>,
) -> LoomResult<i64> {
    let table = meta.table_name();
    let mut sql = format!("SELECT COUNT(*) FROM \"{}\"", table);
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(filters) = filters {
        let mut where_clauses = Vec::new();
        parse_filters_into(filters, meta, &mut bind_values, &mut where_clauses);
        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }
    }

    let mut query = sqlx::query_scalar::<_, i64>(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }

    let count = query.fetch_one(pool).await?;
    Ok(count)
}

/// Check if a document exists.
pub async fn exists(
    pool: &PgPool,
    meta: &Meta,
    filters: &Value,
) -> LoomResult<bool> {
    let count = get_count(pool, meta, Some(filters)).await?;
    Ok(count > 0)
}

// --- Filter parsing ---

/// Parse filters (object or array format) into WHERE clauses and bind values.
///
/// Supported formats:
/// - Object:  `{"field": "value", ...}`  → equality only
/// - Array:   `[["field", "=", "value"], ...]` → any operator
/// - Between: `["field", "between", ["low", "high"]]`
fn parse_filters_into(
    filters: &Value,
    meta: &Meta,
    bind_values: &mut Vec<String>,
    where_clauses: &mut Vec<String>,
) {
    use crate::db::query::FilterOp;

    // Object format: {"field": "value"} — equality
    if let Some(obj) = filters.as_object() {
        for (key, value) in obj {
            let cast = sql_cast_for_key(key, meta);
            bind_values.push(value_to_sql_string(value));
            where_clauses.push(format!(
                "\"{}\" = ${}{}",
                key,
                bind_values.len(),
                cast,
            ));
        }
        return;
    }

    // Array format: [["field", "op", "value"], ...]
    if let Some(arr) = filters.as_array() {
        for item in arr {
            let tuple = match item.as_array() {
                Some(t) => t,
                None => continue,
            };

            if tuple.len() < 3 {
                continue;
            }

            let field = match tuple[0].as_str() {
                Some(f) => f,
                None => continue,
            };
            let op_str = match tuple[1].as_str() {
                Some(o) => o,
                None => continue,
            };
            let value = &tuple[2];

            let op = match FilterOp::from_str(op_str) {
                Some(o) => o,
                None => continue,
            };

            let cast = sql_cast_for_key(field, meta);

            match op {
                FilterOp::IsNull => {
                    where_clauses.push(format!("\"{}\" IS NULL", field));
                }
                FilterOp::IsNotNull => {
                    where_clauses.push(format!("\"{}\" IS NOT NULL", field));
                }
                FilterOp::In | FilterOp::NotIn => {
                    if let Some(vals) = value.as_array() {
                        let placeholders: Vec<String> = vals
                            .iter()
                            .map(|v| {
                                bind_values.push(value_to_sql_string(v));
                                format!("${}{}", bind_values.len(), cast)
                            })
                            .collect();
                        where_clauses.push(format!(
                            "\"{}\" {} ({})",
                            field,
                            op.as_sql(),
                            placeholders.join(", ")
                        ));
                    }
                }
                FilterOp::Between => {
                    if let Some(range) = value.as_array() {
                        if range.len() == 2 {
                            bind_values.push(value_to_sql_string(&range[0]));
                            let lo = bind_values.len();
                            bind_values.push(value_to_sql_string(&range[1]));
                            let hi = bind_values.len();
                            where_clauses.push(format!(
                                "\"{}\" BETWEEN ${}{} AND ${}{}",
                                field, lo, cast, hi, cast
                            ));
                        }
                    }
                }
                FilterOp::Like | FilterOp::NotLike => {
                    bind_values.push(value_to_sql_string(value));
                    where_clauses.push(format!(
                        "\"{}\" {} ${}",
                        field,
                        op.as_sql(),
                        bind_values.len()
                    ));
                }
                _ => {
                    bind_values.push(value_to_sql_string(value));
                    where_clauses.push(format!(
                        "\"{}\" {} ${}{}",
                        field,
                        op.as_sql(),
                        bind_values.len(),
                        cast,
                    ));
                }
            }
        }
    }
}

// --- Helpers ---

// Standard field helpers are in meta.rs:
// super::meta::set_standard_fields_on_insert / set_standard_fields_on_update
use super::meta::{set_standard_fields_on_insert, set_standard_fields_on_update};

fn validate_required_fields(meta: &Meta, doc: &Value) -> LoomResult<()> {
    for field in meta.required_fields() {
        let value = doc.get(&field.fieldname);
        let is_empty = match value {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            _ => false,
        };
        if is_empty {
            return Err(LoomError::Validation(format!(
                "{}: {} is required",
                meta.name,
                field.label.as_deref().unwrap_or(&field.fieldname)
            )));
        }
    }
    Ok(())
}

fn value_to_sql_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "NULL".to_string(),
        _ => value.to_string(),
    }
}

/// Auto-populate fields that have `fetch_from` set.
/// e.g., `fetch_from: "employee.employee_name"` → when `employee` Link is set,
/// fetch `employee_name` from the linked Employee doc and set it on this doc.
pub async fn apply_fetch_from(pool: &PgPool, meta: &Meta, doc: &mut Value) -> LoomResult<()> {
    // Collect fetch_from fields: (target_fieldname, link_fieldname, source_fieldname)
    let fetch_fields: Vec<(&str, String, String)> = meta
        .fields
        .iter()
        .filter_map(|f| {
            let fetch = f.fetch_from.as_deref()?;
            let (link_field, source_field) = fetch.split_once('.')?;
            Some((f.fieldname.as_str(), link_field.to_string(), source_field.to_string()))
        })
        .collect();

    if fetch_fields.is_empty() {
        return Ok(());
    }

    // Group by link field to avoid fetching the same linked doc multiple times
    let mut link_docs: std::collections::HashMap<String, Option<Value>> =
        std::collections::HashMap::new();

    for (_, link_field, _) in &fetch_fields {
        if link_docs.contains_key(link_field) {
            continue;
        }
        let link_value = doc
            .get(link_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if link_value.is_empty() {
            link_docs.insert(link_field.clone(), None);
            continue;
        }

        // Find the Link field's target DocType
        let target_doctype = meta
            .fields
            .iter()
            .find(|f| f.fieldname == *link_field && f.fieldtype == FieldType::Link)
            .and_then(|f| f.options.as_deref());

        let linked_doc = if let Some(dt) = target_doctype {
            let table = super::meta::doctype_table_name(dt);
            let sql = format!(
                "SELECT row_to_json(t.*) FROM \"{}\" t WHERE t.id = $1",
                table
            );
            sqlx::query_scalar::<_, Value>(&sql)
                .bind(&link_value)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
        } else {
            None
        };

        link_docs.insert(link_field.clone(), linked_doc);
    }

    // Apply fetched values
    if let Some(obj) = doc.as_object_mut() {
        for (target_field, link_field, source_field) in &fetch_fields {
            if let Some(Some(linked_doc)) = link_docs.get(link_field) {
                let value = linked_doc.get(source_field).cloned().unwrap_or(Value::Null);
                obj.insert(target_field.to_string(), value);
            } else {
                // Link is empty — clear the fetched field
                obj.insert(target_field.to_string(), Value::Null);
            }
        }
    }

    Ok(())
}

/// Evaluate `mandatory_depends_on` expressions and enforce conditional required fields.
/// The expression is a simple field reference like `eval:doc.is_active` or just a fieldname.
pub fn validate_mandatory_depends_on(meta: &Meta, doc: &Value) -> LoomResult<()> {
    for field in &meta.fields {
        let expr = match &field.mandatory_depends_on {
            Some(e) if !e.is_empty() => e,
            _ => continue,
        };

        // Evaluate the condition — check if the referenced field is truthy
        let is_mandatory = evaluate_simple_condition(expr, doc);

        if !is_mandatory {
            continue;
        }

        // Field is conditionally required — check if it has a value
        let value = doc.get(&field.fieldname);
        let is_empty = match value {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            _ => false,
        };

        if is_empty {
            return Err(LoomError::Validation(format!(
                "{}: {} is required",
                meta.name,
                field.label.as_deref().unwrap_or(&field.fieldname)
            )));
        }
    }
    Ok(())
}

/// Evaluate a simple condition expression against a document.
/// Supports:
///   - `fieldname` — truthy check on a field value
///   - `eval:doc.fieldname` — same, Frappe-style prefix
///   - `eval:doc.fieldname == "value"` — equality check
///   - `eval:doc.fieldname != "value"` — inequality check
fn evaluate_simple_condition(expr: &str, doc: &Value) -> bool {
    let expr = expr.trim();

    // Strip "eval:" prefix and "doc." prefix
    let expr = expr.strip_prefix("eval:").unwrap_or(expr).trim();

    // Check for == or != operators
    if let Some((left, right)) = expr.split_once("==") {
        let field = left.trim().strip_prefix("doc.").unwrap_or(left.trim());
        let expected = right.trim().trim_matches('"').trim_matches('\'');
        let actual = doc.get(field).and_then(|v| v.as_str()).unwrap_or("");
        return actual == expected;
    }
    if let Some((left, right)) = expr.split_once("!=") {
        let field = left.trim().strip_prefix("doc.").unwrap_or(left.trim());
        let expected = right.trim().trim_matches('"').trim_matches('\'');
        let actual = doc.get(field).and_then(|v| v.as_str()).unwrap_or("");
        return actual != expected;
    }

    // Simple truthy check on a field
    let field = expr.strip_prefix("doc.").unwrap_or(expr);
    match doc.get(field) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty() && s != "0",
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(Value::Array(a)) => !a.is_empty(),
        _ => true,
    }
}

/// Validate that all Link field values reference existing documents.
pub async fn validate_link_fields(pool: &PgPool, meta: &Meta, doc: &Value) -> LoomResult<()> {
    for field in meta.fields.iter() {
        if field.fieldtype != FieldType::Link {
            continue;
        }
        let target_doctype = match &field.options {
            Some(dt) if !dt.is_empty() => dt,
            _ => continue,
        };

        let value = match doc.get(&field.fieldname).and_then(|v| v.as_str()) {
            Some(v) if !v.is_empty() => v,
            _ => continue, // Null/empty — skip (required check is separate)
        };

        let target_table = super::meta::doctype_table_name(target_doctype);
        let exists: bool = sqlx::query_scalar(&format!(
            "SELECT EXISTS (SELECT 1 FROM \"{}\" WHERE id = $1)",
            target_table
        ))
        .bind(value)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if !exists {
            return Err(LoomError::LinkValidation {
                doctype: meta.name.clone(),
                fieldname: field.fieldname.clone(),
                value: value.to_string(),
            });
        }
    }
    Ok(())
}

/// Return a SQL cast suffix for bind params that need type coercion.
fn sql_cast_for_key(key: &str, meta: &Meta) -> &'static str {
    // Standard timestamp fields
    match key {
        "creation" | "modified" => return "::TIMESTAMP",
        "docstatus" | "idx" => return "::INTEGER",
        _ => {}
    }
    // Check meta field type
    if let Some(field) = meta.get_field(key) {
        match field.fieldtype {
            super::meta::FieldType::Date => return "::DATE",
            super::meta::FieldType::Datetime => return "::TIMESTAMP",
            super::meta::FieldType::Time => return "::TIME",
            super::meta::FieldType::Int => return "::BIGINT",
            super::meta::FieldType::Float | super::meta::FieldType::Percent => return "::DOUBLE PRECISION",
            super::meta::FieldType::Currency => return "::NUMERIC",
            super::meta::FieldType::Check => return "::BOOLEAN",
            super::meta::FieldType::JSON | super::meta::FieldType::Geolocation => return "::JSONB",
            _ => {}
        }
    }
    ""
}

/// Sanitize ORDER BY clause to prevent SQL injection.
/// Only allows: fieldname, fieldname ASC/DESC, multiple comma-separated.
fn sanitize_order_by(input: &str) -> String {
    input
        .split(',')
        .map(|part| {
            let part = part.trim();
            let tokens: Vec<&str> = part.split_whitespace().collect();
            match tokens.as_slice() {
                [field] => {
                    let f = sanitize_identifier(field);
                    format!("\"{}\"", f)
                }
                [field, dir] => {
                    let f = sanitize_identifier(field);
                    let d = if dir.eq_ignore_ascii_case("asc") { "ASC" } else { "DESC" };
                    format!("\"{}\" {}", f, d)
                }
                _ => "\"modified\" DESC".to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Strip non-alphanumeric/underscore chars from an identifier.
fn sanitize_identifier(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}
