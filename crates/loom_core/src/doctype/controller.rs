use serde_json::Value;

use crate::context::RequestContext;
use crate::db::activity;
use crate::error::{LoomError, LoomResult};
use crate::perms::{
    allowed_permlevels, check_permission, check_write_permlevels, strip_fields_by_permlevel,
    PermType,
};

use super::child;
use super::crud;
use super::hooks::HookEvent;
use super::meta::Meta;
use super::naming::resolve_name;

/// Trait for executing hooks during the document lifecycle.
/// Implemented by RhaiHookRunner (and potentially WASM/compiled hook runners).
#[async_trait::async_trait]
pub trait HookRunner: Send + Sync {
    async fn run_hook(
        &self,
        event: HookEvent,
        doctype: &str,
        doc: &mut Value,
        ctx: &RequestContext,
    ) -> LoomResult<()>;
}

/// A no-op hook runner for when no scripts are loaded.
pub struct NoopHookRunner;

#[async_trait::async_trait]
impl HookRunner for NoopHookRunner {
    async fn run_hook(
        &self,
        _event: HookEvent,
        _doctype: &str,
        _doc: &mut Value,
        _ctx: &RequestContext,
    ) -> LoomResult<()> {
        Ok(())
    }
}

/// Insert a new document with the full lifecycle:
/// permission check → naming → before_insert → validate → before_save → DB insert → after_insert → after_save
pub async fn insert(
    ctx: &RequestContext,
    meta: &Meta,
    doc: &mut Value,
    hooks: &dyn HookRunner,
) -> LoomResult<Value> {
    // Permission check
    if !ctx.ignore_permissions() {
        check_permission(meta, None, PermType::Create, &ctx.user, &ctx.roles)?;
    }

    // Generate name
    let name = resolve_name(meta, doc, ctx.pool()).await?;
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("id".to_string(), Value::String(name));
    }

    // Auto-populate fetch_from fields
    crud::apply_fetch_from(ctx.pool(), meta, doc).await?;

    // Pre-write hooks
    hooks
        .run_hook(HookEvent::BeforeInsert, &meta.name, doc, ctx)
        .await?;
    hooks
        .run_hook(HookEvent::Validate, &meta.name, doc, ctx)
        .await?;
    hooks
        .run_hook(HookEvent::BeforeSave, &meta.name, doc, ctx)
        .await?;

    // Validate Link fields + conditional mandatory fields
    crud::validate_link_fields(ctx.pool(), meta, doc).await?;
    crud::validate_mandatory_depends_on(meta, doc)?;

    // DB insert
    let result = crud::insert_doc(ctx.pool(), meta, doc, &ctx.user).await?;

    // Insert child table rows
    let mut result = result;
    let parent_name = result
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    child::insert_children(
        ctx.pool(),
        meta,
        &parent_name,
        &mut result,
        &ctx.user,
        &ctx.registry,
    )
    .await?;

    // Post-write hooks (operate on a mutable copy)
    hooks
        .run_hook(HookEvent::AfterInsert, &meta.name, &mut result, ctx)
        .await?;
    hooks
        .run_hook(HookEvent::AfterSave, &meta.name, &mut result, ctx)
        .await?;

    // Log activity
    let doc_name = result.get("id").and_then(|v| v.as_str()).unwrap_or("");
    if let Err(e) = activity::log_activity(
        ctx.pool(),
        &meta.name,
        doc_name,
        "Created",
        &ctx.user,
        &serde_json::json!({}),
    )
    .await
    {
        tracing::warn!("Activity log failed: {}", e);
    }

    // Strip fields by read permlevel on response
    if !ctx.ignore_permissions() {
        let read_levels = allowed_permlevels(meta, PermType::Read, &ctx.roles);
        strip_fields_by_permlevel(&mut result, meta, &read_levels);
    }

    Ok(result)
}

/// Get a single document by name with permission check.
pub async fn get(ctx: &RequestContext, meta: &Meta, name: &str) -> LoomResult<Value> {
    let mut doc = crud::get_doc(ctx.pool(), meta, name).await?;

    if !ctx.ignore_permissions() {
        check_permission(meta, Some(&doc), PermType::Read, &ctx.user, &ctx.roles)?;
        let read_levels = allowed_permlevels(meta, PermType::Read, &ctx.roles);
        strip_fields_by_permlevel(&mut doc, meta, &read_levels);
    }

    Ok(doc)
}

/// Get a list of documents with permission check.
pub async fn get_list(
    ctx: &RequestContext,
    meta: &Meta,
    filters: Option<&Value>,
    fields: Option<&[&str]>,
    order_by: Option<&str>,
    limit: Option<u32>,
    offset: Option<u32>,
    search_text: Option<&str>,
) -> LoomResult<Vec<Value>> {
    if !ctx.ignore_permissions() {
        check_permission(meta, None, PermType::Read, &ctx.user, &ctx.roles)?;
    }

    // Build merged filters: original filters + user permission restrictions
    let merged_filters: Option<Value>;
    let effective_filters: Option<&Value>;

    if !ctx.ignore_permissions() {
        let user_perms = crate::perms::get_user_permissions(ctx.pool(), &ctx.user).await?;

        if !user_perms.is_empty() {
            let (perm_clauses, _perm_values) =
                crate::perms::build_user_perm_filters(meta, &user_perms, &meta.name);

            if !perm_clauses.is_empty() {
                // Convert user perm filters into array-format filters and merge
                let mut all_filters: Vec<Value> = Vec::new();

                // Carry over existing filters
                if let Some(existing) = filters {
                    if let Some(arr) = existing.as_array() {
                        all_filters.extend(arr.iter().cloned());
                    } else if let Some(obj) = existing.as_object() {
                        for (k, v) in obj {
                            all_filters.push(serde_json::json!([k, "=", v]));
                        }
                    }
                }

                // Add user perm filters as IN filters
                // perm_clauses are raw SQL, but we need structured filters.
                // Rebuild as ["field", "in", [values]] from the perm data.
                for perm in &user_perms {
                    let applies = match &perm.applicable_for {
                        Some(dt) => dt == &meta.name,
                        None => true,
                    };
                    if !applies {
                        continue;
                    }
                    // Collect all allowed values for this (allow, applicable) combo
                    let allowed: Vec<Value> = user_perms
                        .iter()
                        .filter(|p| {
                            p.allow == perm.allow && p.applicable_for == perm.applicable_for
                        })
                        .map(|p| Value::String(p.for_value.clone()))
                        .collect();

                    if perm.allow == meta.name {
                        // Filter on id
                        all_filters.push(serde_json::json!(["id", "in", allowed]));
                    } else {
                        // Find Link fields pointing to the restricted DocType
                        for field in &meta.fields {
                            if field.fieldtype == super::meta::FieldType::Link {
                                if field.options.as_deref() == Some(&perm.allow) {
                                    all_filters.push(serde_json::json!([
                                        field.fieldname,
                                        "in",
                                        allowed
                                    ]));
                                }
                            }
                        }
                    }
                }

                // Deduplicate: use a set of field names to avoid duplicate IN clauses
                let mut seen_fields = std::collections::HashSet::new();
                all_filters.retain(|f| {
                    if let Some(arr) = f.as_array() {
                        if arr.len() >= 2 {
                            if let (Some(field), Some(op)) = (arr[0].as_str(), arr[1].as_str()) {
                                if op == "in" {
                                    return seen_fields.insert(format!("{}:in", field));
                                }
                            }
                        }
                    }
                    true
                });

                merged_filters = Some(Value::Array(all_filters));
                effective_filters = merged_filters.as_ref();
            } else {
                effective_filters = filters;
            }
        } else {
            effective_filters = filters;
        }
    } else {
        effective_filters = filters;
    }

    let mut docs = crud::get_list(
        ctx.pool(),
        meta,
        effective_filters,
        fields,
        order_by,
        limit,
        offset,
        search_text,
    )
    .await?;

    // Strip fields by read permlevel on each result row
    if !ctx.ignore_permissions() {
        let read_levels = allowed_permlevels(meta, PermType::Read, &ctx.roles);
        for doc in &mut docs {
            strip_fields_by_permlevel(doc, meta, &read_levels);
        }
    }

    Ok(docs)
}

/// Update an existing document with the full lifecycle:
/// permission check → permlevel write check → before_save → validate → DB update → on_update → after_save
pub async fn update(
    ctx: &RequestContext,
    meta: &Meta,
    name: &str,
    doc: &mut Value,
    hooks: &dyn HookRunner,
) -> LoomResult<Value> {
    // Fetch existing doc for permission check and change tracking
    let existing = crud::get_doc(ctx.pool(), meta, name).await?;

    if !ctx.ignore_permissions() {
        check_permission(
            meta,
            Some(&existing),
            PermType::Write,
            &ctx.user,
            &ctx.roles,
        )?;

        // Check write permlevels on incoming fields
        let write_levels = allowed_permlevels(meta, PermType::Write, &ctx.roles);
        check_write_permlevels(doc, meta, &write_levels)?;
    }

    // Build change details: field name, old value, new value
    let changed_details: Vec<Value> = doc
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(key, new_val)| {
                    let old_val = existing.get(key).unwrap_or(&Value::Null);
                    if old_val != new_val {
                        Some(serde_json::json!({
                            "field": key,
                            "from": old_val,
                            "to": new_val,
                        }))
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Auto-populate fetch_from fields
    crud::apply_fetch_from(ctx.pool(), meta, doc).await?;

    // Pre-write hooks
    hooks
        .run_hook(HookEvent::BeforeSave, &meta.name, doc, ctx)
        .await?;
    hooks
        .run_hook(HookEvent::Validate, &meta.name, doc, ctx)
        .await?;

    // Validate Link fields + conditional mandatory fields
    crud::validate_link_fields(ctx.pool(), meta, doc).await?;
    crud::validate_mandatory_depends_on(meta, doc)?;

    // DB update
    let result = crud::update_doc(ctx.pool(), meta, name, doc, &ctx.user).await?;

    // Update child table rows
    let mut result = result;
    child::update_children(ctx.pool(), meta, name, doc, &ctx.user, &ctx.registry).await?;

    // Post-write hooks
    hooks
        .run_hook(HookEvent::OnUpdate, &meta.name, &mut result, ctx)
        .await?;
    hooks
        .run_hook(HookEvent::AfterSave, &meta.name, &mut result, ctx)
        .await?;

    // Log activity with before/after values
    if let Err(e) = activity::log_activity(
        ctx.pool(),
        &meta.name,
        name,
        "Updated",
        &ctx.user,
        &serde_json::json!({ "changed": changed_details }),
    )
    .await
    {
        tracing::warn!("Activity log failed: {}", e);
    }

    // Strip fields by read permlevel on response
    if !ctx.ignore_permissions() {
        let read_levels = allowed_permlevels(meta, PermType::Read, &ctx.roles);
        strip_fields_by_permlevel(&mut result, meta, &read_levels);
    }

    Ok(result)
}

/// Delete a document with the full lifecycle:
/// permission check → on_trash → DB delete
pub async fn delete(
    ctx: &RequestContext,
    meta: &Meta,
    name: &str,
    hooks: &dyn HookRunner,
) -> LoomResult<()> {
    if !ctx.ignore_permissions() {
        let existing = crud::get_doc(ctx.pool(), meta, name).await?;
        check_permission(
            meta,
            Some(&existing),
            PermType::Delete,
            &ctx.user,
            &ctx.roles,
        )?;
    }

    // Pre-delete hook
    let mut doc = crud::get_doc(ctx.pool(), meta, name).await?;
    hooks
        .run_hook(HookEvent::OnTrash, &meta.name, &mut doc, ctx)
        .await?;

    // Delete child table rows first
    child::delete_children(ctx.pool(), meta, name, &ctx.registry).await?;

    // DB delete
    crud::delete_doc(ctx.pool(), meta, name).await
}

/// Submit a document (docstatus 0 → 1):
/// permission check → verify submittable + docstatus=0 → workflow validation → before_submit → validate → set docstatus=1 → on_submit
pub async fn submit(
    ctx: &RequestContext,
    meta: &Meta,
    name: &str,
    hooks: &dyn HookRunner,
) -> LoomResult<Value> {
    if !meta.is_submittable {
        return Err(LoomError::Validation(format!(
            "{} is not submittable",
            meta.name
        )));
    }

    let existing = crud::get_doc(ctx.pool(), meta, name).await?;

    if !ctx.ignore_permissions() {
        check_permission(
            meta,
            Some(&existing),
            PermType::Submit,
            &ctx.user,
            &ctx.roles,
        )?;
    }

    let docstatus = existing
        .get("docstatus")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    if docstatus != 0 {
        return Err(LoomError::Validation(format!(
            "Cannot submit {}: docstatus is {} (expected 0)",
            name, docstatus
        )));
    }

    // Workflow validation
    let mut next_workflow_state: Option<String> = None;
    if let Some(ref workflow) = meta.workflow {
        if workflow.is_active {
            let current_state = existing
                .get("workflow_state")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    workflow
                        .initial_state()
                        .map(|s| s.state.as_str())
                        .unwrap_or("")
                });
            let transition = workflow.validate_transition(current_state, "Submit", &ctx.roles)?;
            next_workflow_state = Some(transition.next_state.clone());
        }
    }

    // Pre-submit hooks
    let mut doc = existing;
    hooks
        .run_hook(HookEvent::BeforeSubmit, &meta.name, &mut doc, ctx)
        .await?;
    hooks
        .run_hook(HookEvent::Validate, &meta.name, &mut doc, ctx)
        .await?;

    // Set docstatus = 1, and workflow_state if applicable
    let mut update_doc = serde_json::json!({ "docstatus": 1 });
    if let Some(ref state) = next_workflow_state {
        update_doc
            .as_object_mut()
            .unwrap()
            .insert("workflow_state".to_string(), Value::String(state.clone()));
    }
    let result = crud::update_doc(ctx.pool(), meta, name, &mut update_doc, &ctx.user).await?;

    // Post-submit hook
    let mut result = result;
    hooks
        .run_hook(HookEvent::OnSubmit, &meta.name, &mut result, ctx)
        .await?;

    // Log activity
    if let Err(e) = activity::log_activity(
        ctx.pool(),
        &meta.name,
        name,
        "Submitted",
        &ctx.user,
        &serde_json::json!({}),
    )
    .await
    {
        tracing::warn!("Activity log failed: {}", e);
    }

    // Strip fields by read permlevel on response
    if !ctx.ignore_permissions() {
        let read_levels = allowed_permlevels(meta, PermType::Read, &ctx.roles);
        strip_fields_by_permlevel(&mut result, meta, &read_levels);
    }

    Ok(result)
}

/// Cancel a submitted document (docstatus 1 → 2):
/// permission check → verify docstatus=1 → workflow validation → before_cancel → set docstatus=2 → on_cancel
pub async fn cancel(
    ctx: &RequestContext,
    meta: &Meta,
    name: &str,
    hooks: &dyn HookRunner,
) -> LoomResult<Value> {
    if !meta.is_submittable {
        return Err(LoomError::Validation(format!(
            "{} is not submittable",
            meta.name
        )));
    }

    let existing = crud::get_doc(ctx.pool(), meta, name).await?;

    if !ctx.ignore_permissions() {
        check_permission(
            meta,
            Some(&existing),
            PermType::Cancel,
            &ctx.user,
            &ctx.roles,
        )?;
    }

    let docstatus = existing
        .get("docstatus")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    if docstatus != 1 {
        return Err(LoomError::Validation(format!(
            "Cannot cancel {}: docstatus is {} (expected 1)",
            name, docstatus
        )));
    }

    // Workflow validation
    let mut next_workflow_state: Option<String> = None;
    if let Some(ref workflow) = meta.workflow {
        if workflow.is_active {
            let current_state = existing
                .get("workflow_state")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let transition = workflow.validate_transition(current_state, "Cancel", &ctx.roles)?;
            next_workflow_state = Some(transition.next_state.clone());
        }
    }

    // Pre-cancel hook
    let mut doc = existing;
    hooks
        .run_hook(HookEvent::BeforeCancel, &meta.name, &mut doc, ctx)
        .await?;

    // Set docstatus = 2, and workflow_state if applicable
    let mut update_doc = serde_json::json!({ "docstatus": 2 });
    if let Some(ref state) = next_workflow_state {
        update_doc
            .as_object_mut()
            .unwrap()
            .insert("workflow_state".to_string(), Value::String(state.clone()));
    }
    let result = crud::update_doc(ctx.pool(), meta, name, &mut update_doc, &ctx.user).await?;

    // Post-cancel hook
    let mut result = result;
    hooks
        .run_hook(HookEvent::OnCancel, &meta.name, &mut result, ctx)
        .await?;

    // Log activity
    if let Err(e) = activity::log_activity(
        ctx.pool(),
        &meta.name,
        name,
        "Cancelled",
        &ctx.user,
        &serde_json::json!({}),
    )
    .await
    {
        tracing::warn!("Activity log failed: {}", e);
    }

    // Strip fields by read permlevel on response
    if !ctx.ignore_permissions() {
        let read_levels = allowed_permlevels(meta, PermType::Read, &ctx.roles);
        strip_fields_by_permlevel(&mut result, meta, &read_levels);
    }

    Ok(result)
}
