//! Integration tests for the DocType controller (full lifecycle with hooks, naming, activity).

use loom_tests::*;
use serde_json::json;

use loom_core::db::activity;
use loom_core::doctype::controller;

#[tokio::test]
async fn test_insert_via_controller() {
    skip_without_db!();
    let meta = test_doctype("Ctrl Insert");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Controller Insert" });
    let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();

    let id = result.get("id").and_then(|v| v.as_str()).unwrap();
    assert!(!id.is_empty());
    assert_eq!(
        result.get("title").and_then(|v| v.as_str()).unwrap(),
        "Controller Insert"
    );
    assert_eq!(
        result.get("owner").and_then(|v| v.as_str()).unwrap(),
        "Administrator"
    );
}

#[tokio::test]
async fn test_get_via_controller() {
    skip_without_db!();
    let meta = test_doctype("Ctrl Get");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Controller Get" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    let fetched = controller::get(&ctx, &meta, id).await.unwrap();
    assert_eq!(
        fetched.get("title").and_then(|v| v.as_str()).unwrap(),
        "Controller Get"
    );
}

#[tokio::test]
async fn test_update_via_controller() {
    skip_without_db!();
    let meta = test_doctype("Ctrl Update");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Before Update" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    let mut update = json!({ "title": "After Update" });
    let updated = controller::update(&ctx, &meta, id, &mut update, &noop_hooks())
        .await
        .unwrap();

    assert_eq!(
        updated.get("title").and_then(|v| v.as_str()).unwrap(),
        "After Update"
    );
}

#[tokio::test]
async fn test_delete_via_controller() {
    skip_without_db!();
    let meta = test_doctype("Ctrl Delete");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "To Delete" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    controller::delete(&ctx, &meta, id, &noop_hooks())
        .await
        .unwrap();

    let result = controller::get(&ctx, &meta, id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_list_via_controller() {
    skip_without_db!();
    let meta = test_doctype("Ctrl List");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    for i in 1..=3 {
        let mut doc = json!({ "title": format!("List Item {}", i) });
        controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
            .await
            .unwrap();
    }

    let list = controller::get_list(&ctx, &meta, None, None, None, Some(10), None, None)
        .await
        .unwrap();
    assert_eq!(list.len(), 3);
}

#[tokio::test]
async fn test_submit_lifecycle() {
    skip_without_db!();
    let meta = test_submittable_doctype("Ctrl Submit");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Submittable Doc" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Verify docstatus is 0 (Draft)
    assert_eq!(
        inserted
            .get("docstatus")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1),
        0
    );

    // Submit
    let submitted = controller::submit(&ctx, &meta, id, &noop_hooks())
        .await
        .unwrap();
    assert_eq!(
        submitted
            .get("docstatus")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1),
        1
    );
}

#[tokio::test]
async fn test_cancel_lifecycle() {
    skip_without_db!();
    let meta = test_submittable_doctype("Ctrl Cancel");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Cancel Me" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    controller::submit(&ctx, &meta, id, &noop_hooks())
        .await
        .unwrap();
    let cancelled = controller::cancel(&ctx, &meta, id, &noop_hooks())
        .await
        .unwrap();

    assert_eq!(
        cancelled
            .get("docstatus")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1),
        2
    );
}

#[tokio::test]
async fn test_submit_non_submittable_fails() {
    skip_without_db!();
    let meta = test_doctype("Ctrl NonSubmit");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Not Submittable" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    let result = controller::submit(&ctx, &meta, id, &noop_hooks()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cancel_unsubmitted_fails() {
    skip_without_db!();
    let meta = test_submittable_doctype("Ctrl CancelFail");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Not Yet Submitted" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Cancel without submitting should fail (docstatus is 0, not 1)
    let result = controller::cancel(&ctx, &meta, id, &noop_hooks()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_activity_logged_on_insert() {
    skip_without_db!();
    let meta = test_doctype("Ctrl Activity");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Activity Test" });
    let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = result.get("id").and_then(|v| v.as_str()).unwrap();

    let timeline = activity::get_activity(&db.pool, &meta.name, id, 10)
        .await
        .unwrap();
    assert!(!timeline.is_empty());
    assert_eq!(
        timeline[0].get("action").and_then(|v| v.as_str()).unwrap(),
        "Created"
    );
}

#[tokio::test]
async fn test_activity_logged_on_update() {
    skip_without_db!();
    let meta = test_doctype("Ctrl ActUpd");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "Before" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    let mut update = json!({ "title": "After" });
    controller::update(&ctx, &meta, id, &mut update, &noop_hooks())
        .await
        .unwrap();

    let timeline = activity::get_activity(&db.pool, &meta.name, id, 10)
        .await
        .unwrap();
    assert!(timeline.len() >= 2);

    // Most recent should be "Updated"
    assert_eq!(
        timeline[0].get("action").and_then(|v| v.as_str()).unwrap(),
        "Updated"
    );
}

#[tokio::test]
async fn test_hash_naming_generates_unique_ids() {
    skip_without_db!();
    let meta = test_doctype("Ctrl Naming");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut ids = Vec::new();
    for i in 1..=5 {
        let mut doc = json!({ "title": format!("Name Test {}", i) });
        let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
            .await
            .unwrap();
        let id = result
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();
        assert!(!ids.contains(&id), "Duplicate ID generated: {}", id);
        ids.push(id);
    }
    assert_eq!(ids.len(), 5);
}

#[tokio::test]
async fn test_fetch_from_linked_doc() {
    skip_without_db!();
    let parent_meta = test_doctype("Ctrl Parent");
    let child_meta = test_linked_doctype("Ctrl Child", "Ctrl Parent");
    let (db, registry) = TestDb::with_doctypes(vec![parent_meta.clone(), child_meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    // Insert parent doc
    let mut parent = json!({ "title": "Parent Title" });
    let parent_result = controller::insert(&ctx, &parent_meta, &mut parent, &noop_hooks())
        .await
        .unwrap();
    let parent_id = parent_result.get("id").and_then(|v| v.as_str()).unwrap();

    // Insert child doc linking to parent
    let mut child_doc = json!({
        "title": "Child Title",
        "linked_item": parent_id,
    });
    let child_result = controller::insert(&ctx, &child_meta, &mut child_doc, &noop_hooks())
        .await
        .unwrap();

    // fetch_from should have populated linked_title
    assert_eq!(
        child_result
            .get("linked_title")
            .and_then(|v| v.as_str())
            .unwrap(),
        "Parent Title"
    );
}
