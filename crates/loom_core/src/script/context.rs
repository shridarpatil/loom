use rhai::{Dynamic, Engine};
use sqlx::PgPool;
use std::sync::Arc;

use crate::doctype::DocTypeRegistry;

/// Register the full `loom.*` API into the Rhai engine.
pub fn register_db_api(
    engine: &mut Engine,
    pool: Arc<PgPool>,
    registry: Arc<DocTypeRegistry>,
    user: String,
    roles: Vec<String>,
) {
    register_core_api(
        engine,
        pool.clone(),
        registry.clone(),
        user.clone(),
        roles.clone(),
    );

    // loom_call — only in the outer version (not inner, to prevent infinite recursion)
    {
        let pool = pool.clone();
        let registry = registry.clone();
        let user = user.clone();
        let roles = roles.clone();
        engine.register_fn("loom_call", move |method: &str, args: Dynamic| -> Dynamic {
            let pool = pool.clone();
            let registry = registry.clone();
            let user = user.clone();
            let roles = roles.clone();
            let method = method.to_string();

            let (app_name, method_name) = match method.split_once('.') {
                Some((a, m)) => (a.to_string(), m.to_string()),
                None => {
                    tracing::error!("loom_call: method path must be 'app.method_name'");
                    return Dynamic::UNIT;
                }
            };

            let script_path = format!("apps/{}/api/{}.rhai", app_name, method_name);
            let source = match std::fs::read_to_string(&script_path) {
                Ok(s) => s,
                Err(_) => {
                    tracing::error!("loom_call: script not found at {}", script_path);
                    return Dynamic::UNIT;
                }
            };

            // Sub-engine gets core API but NOT loom_call (prevents infinite recursion)
            let mut sub_engine = crate::script::create_engine();
            crate::script::api::register_loom_api(&mut sub_engine);
            register_core_api(&mut sub_engine, pool, registry, user, roles);

            let ast = match sub_engine.compile(&source) {
                Ok(a) => a,
                Err(e) => {
                    tracing::error!("loom_call compile error: {}", e);
                    return Dynamic::UNIT;
                }
            };

            if !ast.iter_functions().any(|f| f.name == "main") {
                tracing::error!("loom_call: '{}' has no main() function", method);
                return Dynamic::UNIT;
            }

            let mut scope = rhai::Scope::new();
            let loom_map = Dynamic::from(rhai::Map::new());
            match sub_engine.call_fn::<Dynamic>(&mut scope, &ast, "main", (args, loom_map)) {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("loom_call error: {}", e);
                    Dynamic::UNIT
                }
            }
        });
    }
}

/// Register all loom API functions except loom_call.
/// Shared by both `register_db_api` (outer) and loom_call's sub-engine (inner).
fn register_core_api(
    engine: &mut Engine,
    pool: Arc<PgPool>,
    registry: Arc<DocTypeRegistry>,
    user: String,
    roles: Vec<String>,
) {
    register_session_api(engine, user.clone(), roles.clone());
    register_db_read_api(engine, pool.clone(), registry.clone());
    register_db_write_api(engine, pool.clone(), registry.clone(), user.clone());
    register_permission_api(engine, registry.clone(), user, roles);
    register_date_api(engine);
    register_enqueue_api(engine, pool);
    register_sendmail_api(engine);
}

// =========================================================================
// Session
// =========================================================================

fn register_session_api(engine: &mut Engine, user: String, roles: Vec<String>) {
    {
        let u = user;
        engine.register_fn("loom_session_user", move || -> String { u.clone() });
    }
    {
        let r = roles;
        engine.register_fn("loom_session_roles", move || -> Vec<Dynamic> {
            r.iter().map(|s| Dynamic::from(s.clone())).collect()
        });
    }
}

// =========================================================================
// DB Reads
// =========================================================================

fn register_db_read_api(engine: &mut Engine, pool: Arc<PgPool>, registry: Arc<DocTypeRegistry>) {
    // get_doc
    {
        let pool = pool.clone();
        engine.register_fn(
            "loom_db_get_doc",
            move |doctype: &str, name: &str| -> Dynamic {
                let pool = pool.clone();
                let doctype = doctype.to_string();
                let name = name.to_string();
                block_on(async move {
                    let table = crate::doctype::meta::doctype_table_name(&doctype);
                    let sql = format!(
                        "SELECT row_to_json(t.*) FROM \"{}\" t WHERE t.id = $1",
                        table
                    );
                    let row: Option<serde_json::Value> = sqlx::query_scalar(&sql)
                        .bind(&name)
                        .fetch_optional(pool.as_ref())
                        .await
                        .unwrap_or(None);
                    match row {
                        Some(val) => rhai::serde::to_dynamic(&val).unwrap_or(Dynamic::UNIT),
                        None => Dynamic::UNIT,
                    }
                })
            },
        );
    }

    // get_value
    {
        let pool = pool.clone();
        engine.register_fn(
            "loom_db_get_value",
            move |doctype: &str, filters: Dynamic, fieldname: &str| -> Dynamic {
                let pool = pool.clone();
                let doctype = doctype.to_string();
                let fieldname = fieldname.to_string();
                let filters_val: serde_json::Value =
                    rhai::serde::from_dynamic(&filters).unwrap_or(serde_json::Value::Null);
                block_on(async move {
                    match crate::db::engine::get_value(
                        pool.as_ref(),
                        &doctype,
                        &filters_val,
                        &fieldname,
                    )
                    .await
                    {
                        Ok(Some(val)) => rhai::serde::to_dynamic(&val).unwrap_or(Dynamic::UNIT),
                        _ => Dynamic::UNIT,
                    }
                })
            },
        );
    }

    // get_all
    {
        let pool = pool.clone();
        let registry = registry.clone();
        engine.register_fn(
            "loom_db_get_all",
            move |doctype: &str, options: Dynamic| -> Dynamic {
                let pool = pool.clone();
                let registry = registry.clone();
                let doctype = doctype.to_string();
                let opts: serde_json::Value = rhai::serde::from_dynamic(&options)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                block_on(async move {
                    let meta = match registry.get_meta(&doctype).await {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::error!("loom_db_get_all: {}", e);
                            return Dynamic::from(Vec::<Dynamic>::new());
                        }
                    };
                    let filters = opts.get("filters");
                    let order_by = opts.get("order_by").and_then(|v| v.as_str());
                    let limit = opts.get("limit").and_then(|v| v.as_u64()).map(|v| v as u32);
                    let field_strs: Option<Vec<String>> = opts.get("fields").and_then(|v| {
                        v.as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|f| f.as_str().map(String::from))
                                .collect()
                        })
                    });
                    let field_refs: Option<Vec<&str>> = field_strs
                        .as_ref()
                        .map(|v| v.iter().map(|s| s.as_str()).collect());
                    match crate::doctype::crud::get_list(
                        pool.as_ref(),
                        &meta,
                        filters,
                        field_refs.as_deref(),
                        order_by,
                        limit,
                        None,
                        None,
                    )
                    .await
                    {
                        Ok(rows) => Dynamic::from(
                            rows.into_iter()
                                .filter_map(|v| rhai::serde::to_dynamic(&v).ok())
                                .collect::<Vec<Dynamic>>(),
                        ),
                        Err(e) => {
                            tracing::error!("loom_db_get_all error: {}", e);
                            Dynamic::from(Vec::<Dynamic>::new())
                        }
                    }
                })
            },
        );
    }

    // exists
    {
        let pool = pool.clone();
        engine.register_fn(
            "loom_db_exists",
            move |doctype: &str, filters: Dynamic| -> bool {
                let pool = pool.clone();
                let doctype = doctype.to_string();
                let filters_val: serde_json::Value =
                    rhai::serde::from_dynamic(&filters).unwrap_or(serde_json::Value::Null);
                block_on(async move {
                    crate::db::engine::exists(pool.as_ref(), &doctype, &filters_val)
                        .await
                        .unwrap_or(false)
                })
            },
        );
    }

    // count
    {
        let pool = pool.clone();
        engine.register_fn(
            "loom_db_count",
            move |doctype: &str, filters: Dynamic| -> i64 {
                let pool = pool.clone();
                let doctype = doctype.to_string();
                let filters_val: serde_json::Value =
                    rhai::serde::from_dynamic(&filters).unwrap_or(serde_json::Value::Null);
                block_on(async move {
                    crate::db::engine::count(pool.as_ref(), &doctype, Some(&filters_val))
                        .await
                        .unwrap_or(0)
                })
            },
        );
    }

    // sql (read-only)
    {
        let pool = pool.clone();
        engine.register_fn(
            "loom_db_sql",
            move |query: &str, params: Dynamic| -> Dynamic {
                let pool = pool.clone();
                let query = query.to_string();
                let params_val: serde_json::Value =
                    rhai::serde::from_dynamic(&params).unwrap_or(serde_json::Value::Array(vec![]));

                let trimmed = query.trim_start().to_uppercase();
                if !trimmed.starts_with("SELECT") {
                    tracing::error!("loom_db_sql: only SELECT queries are allowed");
                    return Dynamic::from(Vec::<Dynamic>::new());
                }

                block_on(async move {
                    let bind_vals: Vec<String> = match params_val.as_array() {
                        Some(arr) => arr
                            .iter()
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            })
                            .collect(),
                        None => vec![],
                    };
                    let wrapped = format!("SELECT row_to_json(t) FROM ({}) t", query);
                    let mut q = sqlx::query_scalar::<_, serde_json::Value>(&wrapped);
                    for val in &bind_vals {
                        q = q.bind(val);
                    }
                    match q.fetch_all(pool.as_ref()).await {
                        Ok(rows) => Dynamic::from(
                            rows.into_iter()
                                .filter_map(|v| rhai::serde::to_dynamic(&v).ok())
                                .collect::<Vec<Dynamic>>(),
                        ),
                        Err(e) => {
                            tracing::error!("loom_db_sql error: {}", e);
                            Dynamic::from(Vec::<Dynamic>::new())
                        }
                    }
                })
            },
        );
    }
}

// =========================================================================
// DB Writes
// =========================================================================

fn register_db_write_api(
    engine: &mut Engine,
    pool: Arc<PgPool>,
    registry: Arc<DocTypeRegistry>,
    user: String,
) {
    // set_value
    {
        let pool = pool.clone();
        engine.register_fn(
            "loom_db_set_value",
            move |doctype: &str, name: &str, field: &str, value: Dynamic| {
                let pool = pool.clone();
                let doctype = doctype.to_string();
                let name = name.to_string();
                let field = field.to_string();
                let val: serde_json::Value =
                    rhai::serde::from_dynamic(&value).unwrap_or(serde_json::Value::Null);
                block_on(async move {
                    if let Err(e) =
                        crate::db::engine::set_value(pool.as_ref(), &doctype, &name, &field, &val)
                            .await
                    {
                        tracing::error!("loom_db_set_value error: {}", e);
                    }
                });
            },
        );
    }

    // add_value
    {
        let pool = pool.clone();
        engine.register_fn("loom_db_add_value", move |doctype: &str, name: &str, field: &str, amount: Dynamic| {
            let pool = pool.clone();
            let doctype = doctype.to_string();
            let name = name.to_string();
            let field = field.to_string();
            let amt: f64 = if amount.is_int() { amount.as_int().unwrap_or(0) as f64 } else { amount.as_float().unwrap_or(0.0) };
            block_on(async move {
                let table = crate::doctype::meta::doctype_table_name(&doctype);
                let sql = format!("UPDATE \"{}\" SET \"{}\" = COALESCE(\"{}\", 0) + $1, modified = NOW() WHERE id = $2", table, field, field);
                if let Err(e) = sqlx::query(&sql).bind(amt).bind(&name).execute(pool.as_ref()).await {
                    tracing::error!("loom_db_add_value error: {}", e);
                }
            });
        });
    }

    // insert
    {
        let pool = pool.clone();
        let registry = registry.clone();
        let user = user.clone();
        engine.register_fn("loom_db_insert", move |doc: Dynamic| -> Dynamic {
            let pool = pool.clone();
            let registry = registry.clone();
            let user = user.clone();
            let mut doc_val: serde_json::Value =
                rhai::serde::from_dynamic(&doc).unwrap_or(serde_json::Value::Null);
            block_on(async move {
                let doctype = doc_val
                    .get("doctype")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if doctype.is_empty() {
                    tracing::error!("loom_db_insert: missing doctype field");
                    return Dynamic::UNIT;
                }
                if let Some(o) = doc_val.as_object_mut() {
                    o.remove("doctype");
                }
                let meta = match registry.get_meta(&doctype).await {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!("loom_db_insert: {}", e);
                        return Dynamic::UNIT;
                    }
                };
                if doc_val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .is_empty()
                {
                    match crate::doctype::naming::resolve_name(&meta, &doc_val, pool.as_ref()).await
                    {
                        Ok(name) => {
                            if let Some(o) = doc_val.as_object_mut() {
                                o.insert("id".to_string(), serde_json::Value::String(name));
                            }
                        }
                        Err(e) => {
                            tracing::error!("loom_db_insert naming error: {}", e);
                            return Dynamic::UNIT;
                        }
                    }
                }
                match crate::doctype::crud::insert_doc(pool.as_ref(), &meta, &mut doc_val, &user)
                    .await
                {
                    Ok(result) => rhai::serde::to_dynamic(&result).unwrap_or(Dynamic::UNIT),
                    Err(e) => {
                        tracing::error!("loom_db_insert error: {}", e);
                        Dynamic::UNIT
                    }
                }
            })
        });
    }

    // delete
    {
        let pool = pool.clone();
        engine.register_fn("loom_db_delete", move |doctype: &str, name: &str| {
            let pool = pool.clone();
            let doctype = doctype.to_string();
            let name = name.to_string();
            block_on(async move {
                let table = crate::doctype::meta::doctype_table_name(&doctype);
                let sql = format!("DELETE FROM \"{}\" WHERE id = $1", table);
                if let Err(e) = sqlx::query(&sql).bind(&name).execute(pool.as_ref()).await {
                    tracing::error!("loom_db_delete error: {}", e);
                }
            });
        });
    }
}

// =========================================================================
// Permissions
// =========================================================================

fn register_permission_api(
    engine: &mut Engine,
    registry: Arc<DocTypeRegistry>,
    user: String,
    roles: Vec<String>,
) {
    // has_permission
    {
        let registry = registry.clone();
        let user = user.clone();
        let roles = roles.clone();
        engine.register_fn(
            "loom_has_permission",
            move |doctype: &str, ptype: &str| -> bool {
                let registry = registry.clone();
                let user = user.clone();
                let roles = roles.clone();
                let doctype = doctype.to_string();
                let ptype = ptype.to_string();
                block_on(async move {
                    let meta = match registry.get_meta(&doctype).await {
                        Ok(m) => m,
                        Err(_) => return false,
                    };
                    let pt = match parse_perm_type(&ptype) {
                        Some(p) => p,
                        None => return false,
                    };
                    crate::perms::has_permission(&meta, None, pt, &user, &roles)
                })
            },
        );
    }

    // check_permission
    {
        let registry = registry.clone();
        let user = user.clone();
        let roles = roles.clone();
        engine.register_fn(
            "loom_check_permission",
            move |doctype: &str, ptype: &str| -> Result<(), Box<rhai::EvalAltResult>> {
                let registry = registry.clone();
                let user = user.clone();
                let roles = roles.clone();
                let doctype = doctype.to_string();
                let ptype = ptype.to_string();
                block_on(async move {
                    let meta = match registry.get_meta(&doctype).await {
                        Ok(m) => m,
                        Err(e) => return Err(e.to_string().into()),
                    };
                    let pt = match parse_perm_type(&ptype) {
                        Some(p) => p,
                        None => return Err(format!("Unknown permission type: {}", ptype).into()),
                    };
                    match crate::perms::check_permission(&meta, None, pt, &user, &roles) {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e.to_string().into()),
                    }
                })
            },
        );
    }
}

// =========================================================================
// Date helpers
// =========================================================================

fn register_date_api(engine: &mut Engine) {
    engine.register_fn("loom_add_days", |date: &str, days: i64| -> String {
        match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(d) => (d + chrono::Duration::days(days))
                .format("%Y-%m-%d")
                .to_string(),
            Err(_) => date.to_string(),
        }
    });

    engine.register_fn("loom_add_months", |date: &str, months: i64| -> String {
        match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(d) => {
                let total_months = d.year() as i64 * 12 + d.month0() as i64 + months;
                let new_year = (total_months / 12) as i32;
                let new_month = (total_months % 12) as u32 + 1;
                let new_day = d.day().min(days_in_month(new_year, new_month));
                chrono::NaiveDate::from_ymd_opt(new_year, new_month, new_day)
                    .unwrap_or(d)
                    .format("%Y-%m-%d")
                    .to_string()
            }
            Err(_) => date.to_string(),
        }
    });
}

// =========================================================================
// Enqueue
// =========================================================================

fn register_enqueue_api(engine: &mut Engine, pool: Arc<PgPool>) {
    {
        let pool = pool.clone();
        engine.register_fn("loom_enqueue", move |method: &str, args: Dynamic| {
            enqueue_job(pool.clone(), method, &args, Dynamic::UNIT);
        });
    }
    {
        let pool = pool.clone();
        engine.register_fn(
            "loom_enqueue",
            move |method: &str, args: Dynamic, options: Dynamic| {
                enqueue_job(pool.clone(), method, &args, options);
            },
        );
    }
}

// =========================================================================
// Sendmail
// =========================================================================

fn register_sendmail_api(engine: &mut Engine) {
    engine.register_fn("loom_sendmail", |options: Dynamic| {
        let opts: serde_json::Value =
            rhai::serde::from_dynamic(&options).unwrap_or(serde_json::Value::Null);
        tracing::warn!("loom_sendmail called (not yet implemented): {:?}", opts);
    });
}

// =========================================================================
// Helpers
// =========================================================================

/// Bridge async code into synchronous Rhai closures.
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
}

fn enqueue_job(pool: Arc<PgPool>, method: &str, args: &Dynamic, options: Dynamic) {
    let method = method.to_string();
    let args_val: serde_json::Value =
        rhai::serde::from_dynamic(args).unwrap_or(serde_json::Value::Object(Default::default()));
    let opts_val: serde_json::Value =
        rhai::serde::from_dynamic(&options).unwrap_or(serde_json::Value::Null);

    let queue = opts_val
        .get("queue")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let priority = opts_val
        .get("priority")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let max_retries = opts_val
        .get("max_retries")
        .and_then(|v| v.as_i64())
        .unwrap_or(3) as i32;

    block_on(async {
        let result = sqlx::query(
            "INSERT INTO \"__job_queue\" (method, args, queue, priority, status, attempts, max_retries, created) \
             VALUES ($1, $2, $3, $4, 'queued', 0, $5, NOW())",
        )
        .bind(&method)
        .bind(&args_val)
        .bind(&queue)
        .bind(priority)
        .bind(max_retries)
        .execute(pool.as_ref())
        .await;

        match result {
            Ok(_) => tracing::info!(
                "Enqueued '{}' on queue '{}' (priority {})",
                method,
                queue,
                priority
            ),
            Err(e) => tracing::error!("loom_enqueue error: {}", e),
        }
    });
}

use chrono::Datelike;

fn days_in_month(year: i32, month: u32) -> u32 {
    chrono::NaiveDate::from_ymd_opt(
        if month == 12 { year + 1 } else { year },
        if month == 12 { 1 } else { month + 1 },
        1,
    )
    .unwrap_or(chrono::NaiveDate::from_ymd_opt(year, month, 28).unwrap())
    .pred_opt()
    .unwrap()
    .day()
}

fn parse_perm_type(s: &str) -> Option<crate::perms::PermType> {
    match s.to_lowercase().as_str() {
        "read" => Some(crate::perms::PermType::Read),
        "write" => Some(crate::perms::PermType::Write),
        "create" => Some(crate::perms::PermType::Create),
        "delete" => Some(crate::perms::PermType::Delete),
        "submit" => Some(crate::perms::PermType::Submit),
        "cancel" => Some(crate::perms::PermType::Cancel),
        "amend" => Some(crate::perms::PermType::Amend),
        "report" => Some(crate::perms::PermType::Report),
        "export" => Some(crate::perms::PermType::Export),
        "print" => Some(crate::perms::PermType::Print),
        "email" => Some(crate::perms::PermType::Email),
        "share" => Some(crate::perms::PermType::Share),
        _ => None,
    }
}
