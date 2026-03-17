//! Integration tests for Rhai hook execution with a real database using RhaiHookRunner.

use std::sync::Arc;

use loom_tests::*;
use serde_json::json;

use loom_core::doctype::controller;
use loom_core::doctype::RhaiHookRunner;
use loom_core::script::{create_engine, ScriptCache};

/// Create a RhaiHookRunner for testing.
async fn make_hook_runner() -> RhaiHookRunner {
    let engine = Arc::new(create_engine());
    let cache = ScriptCache::new();
    RhaiHookRunner::new(engine, cache)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_validate_hook_rejects_invalid_doc() {
    skip_without_db!();
    let meta = test_doctype("Hook Validate");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let hooks = make_hook_runner().await;

    // Load a script that validates title is not empty
    let script = r#"
fn validate(doc) {
    if doc.title == "" {
        throw("Title is required and cannot be empty");
    }
}
"#;
    hooks.load_script("Hook Validate", script.to_string()).await;

    // Insert doc with empty title — should fail validation
    let mut doc = json!({ "title": "" });
    let result = controller::insert(&ctx, &meta, &mut doc, &hooks).await;

    assert!(result.is_err(), "Insert with empty title should fail");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Title is required"),
        "Error should mention title validation: {}",
        err_msg
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_before_save_hook_modifies_doc() {
    skip_without_db!();
    let meta = test_doctype("Hook BeforeSave");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let hooks = make_hook_runner().await;

    // Load a script that sets status = "Processed" in before_save
    let script = r#"
fn before_save(doc) {
    doc.status = "Processed";
    doc
}
"#;
    hooks
        .load_script("Hook BeforeSave", script.to_string())
        .await;

    let mut doc = json!({ "title": "My Doc", "status": "Open" });
    let result = controller::insert(&ctx, &meta, &mut doc, &hooks)
        .await
        .unwrap();

    // The before_save hook should have changed status to "Processed"
    // However, the hook modifies the doc *before* the DB write, so the
    // persisted value should reflect the hook's change.
    let id = result.get("id").and_then(|v| v.as_str()).unwrap();
    let fetched = controller::get(&ctx, &meta, id).await.unwrap();

    assert_eq!(
        fetched.get("status").and_then(|v| v.as_str()).unwrap(),
        "Processed",
        "before_save hook should have set status to Processed"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_hook_no_function_skips() {
    skip_without_db!();
    let meta = test_doctype("Hook NoFn");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let hooks = make_hook_runner().await;

    // Load a script that defines a helper function but NOT validate/before_save
    let script = r#"
fn some_other_function(x) {
    x + 1
}
"#;
    hooks.load_script("Hook NoFn", script.to_string()).await;

    // Insert should succeed — missing hook functions are silently skipped
    let mut doc = json!({ "title": "No Hook Function" });
    let result = controller::insert(&ctx, &meta, &mut doc, &hooks).await;

    assert!(
        result.is_ok(),
        "Insert should succeed when script has no matching hook function"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_hook_no_script_skips() {
    skip_without_db!();
    let meta = test_doctype("Hook NoScript");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let hooks = make_hook_runner().await;

    // Do NOT load any script for this DocType

    // Insert should succeed — no script means no hooks to run
    let mut doc = json!({ "title": "No Script Loaded" });
    let result = controller::insert(&ctx, &meta, &mut doc, &hooks).await;

    assert!(
        result.is_ok(),
        "Insert should succeed when no script is loaded for the DocType"
    );
    let inserted = result.unwrap();
    assert_eq!(
        inserted.get("title").and_then(|v| v.as_str()).unwrap(),
        "No Script Loaded"
    );
}
