//! Integration tests for the activity/audit trail system.

use loom_tests::*;
use serde_json::json;

use loom_core::db::activity;

#[tokio::test]
async fn test_log_and_get_activity() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    activity::log_activity(
        &db.pool,
        "Test DocType",
        "DOC-001",
        "Created",
        "admin@test.com",
        &json!({}),
    )
    .await
    .unwrap();

    let timeline = activity::get_activity(&db.pool, "Test DocType", "DOC-001", 10)
        .await
        .unwrap();

    assert_eq!(timeline.len(), 1);
    assert_eq!(
        timeline[0].get("action").and_then(|v| v.as_str()).unwrap(),
        "Created"
    );
    assert_eq!(
        timeline[0].get("user").and_then(|v| v.as_str()).unwrap(),
        "admin@test.com"
    );
}

#[tokio::test]
async fn test_multiple_activities_ordered() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    activity::log_activity(
        &db.pool,
        "Invoice",
        "INV-001",
        "Created",
        "admin@test.com",
        &json!({}),
    )
    .await
    .unwrap();

    activity::log_activity(
        &db.pool,
        "Invoice",
        "INV-001",
        "Updated",
        "admin@test.com",
        &json!({ "changed": [{"field": "amount", "from": "100", "to": "200"}] }),
    )
    .await
    .unwrap();

    activity::log_activity(
        &db.pool,
        "Invoice",
        "INV-001",
        "Submitted",
        "admin@test.com",
        &json!({}),
    )
    .await
    .unwrap();

    let timeline = activity::get_activity(&db.pool, "Invoice", "INV-001", 10)
        .await
        .unwrap();

    assert_eq!(timeline.len(), 3);
    // Most recent first
    assert_eq!(
        timeline[0].get("action").and_then(|v| v.as_str()).unwrap(),
        "Submitted"
    );
    assert_eq!(
        timeline[1].get("action").and_then(|v| v.as_str()).unwrap(),
        "Updated"
    );
    assert_eq!(
        timeline[2].get("action").and_then(|v| v.as_str()).unwrap(),
        "Created"
    );
}

#[tokio::test]
async fn test_activity_with_data() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    let data = json!({
        "changed": [
            { "field": "title", "from": "Old", "to": "New" },
            { "field": "status", "from": "Open", "to": "Closed" }
        ]
    });

    activity::log_activity(
        &db.pool,
        "Task",
        "TASK-001",
        "Updated",
        "user@test.com",
        &data,
    )
    .await
    .unwrap();

    let timeline = activity::get_activity(&db.pool, "Task", "TASK-001", 10)
        .await
        .unwrap();

    let entry_data = timeline[0].get("data").unwrap();
    let changed = entry_data
        .get("changed")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(changed.len(), 2);
}

#[tokio::test]
async fn test_add_comment() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    activity::add_comment(
        &db.pool,
        "Task",
        "TASK-001",
        "user@test.com",
        "This looks good, approved!",
    )
    .await
    .unwrap();

    let timeline = activity::get_activity(&db.pool, "Task", "TASK-001", 10)
        .await
        .unwrap();

    assert_eq!(timeline.len(), 1);
    assert_eq!(
        timeline[0].get("action").and_then(|v| v.as_str()).unwrap(),
        "Commented"
    );
    let comment = timeline[0]
        .get("data")
        .and_then(|d| d.get("comment"))
        .and_then(|v| v.as_str())
        .unwrap();
    assert_eq!(comment, "This looks good, approved!");
}

#[tokio::test]
async fn test_activity_isolation_between_docs() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    activity::log_activity(
        &db.pool,
        "Task",
        "TASK-001",
        "Created",
        "admin@test.com",
        &json!({}),
    )
    .await
    .unwrap();

    activity::log_activity(
        &db.pool,
        "Task",
        "TASK-002",
        "Created",
        "admin@test.com",
        &json!({}),
    )
    .await
    .unwrap();

    // Each doc should only see its own activity
    let timeline1 = activity::get_activity(&db.pool, "Task", "TASK-001", 10)
        .await
        .unwrap();
    assert_eq!(timeline1.len(), 1);

    let timeline2 = activity::get_activity(&db.pool, "Task", "TASK-002", 10)
        .await
        .unwrap();
    assert_eq!(timeline2.len(), 1);
}

#[tokio::test]
async fn test_activity_limit() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    for i in 1..=5 {
        activity::log_activity(
            &db.pool,
            "Task",
            "TASK-LIMIT",
            &format!("Action {}", i),
            "admin@test.com",
            &json!({}),
        )
        .await
        .unwrap();
    }

    // Limit to 3
    let timeline = activity::get_activity(&db.pool, "Task", "TASK-LIMIT", 3)
        .await
        .unwrap();
    assert_eq!(timeline.len(), 3);
}
