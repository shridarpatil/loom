//! Integration tests for the background job queue.

use loom_tests::*;
use serde_json::json;

use loom_queue::queue::{self, EnqueueOptions};

#[tokio::test]
async fn test_enqueue_returns_id() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    let id = queue::enqueue(
        &db.pool,
        "test_app.test_method",
        &json!({ "key": "value" }),
        Default::default(),
    )
    .await
    .unwrap();

    assert!(id > 0);
}

#[tokio::test]
async fn test_enqueue_dequeue_cycle() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    queue::enqueue(
        &db.pool,
        "app.do_work",
        &json!({ "task_id": 42 }),
        Default::default(),
    )
    .await
    .unwrap();

    let job = queue::dequeue(&db.pool, "default").await.unwrap();
    assert!(job.is_some());

    let job = job.unwrap();
    assert_eq!(job.method, "app.do_work");
    assert_eq!(job.args.get("task_id").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(job.status, "running");
    assert_eq!(job.attempts, 1);
}

#[tokio::test]
async fn test_dequeue_empty_queue() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    let job = queue::dequeue(&db.pool, "default").await.unwrap();
    assert!(job.is_none());
}

#[tokio::test]
async fn test_priority_ordering() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    // Enqueue low priority first
    queue::enqueue(
        &db.pool,
        "app.low",
        &json!({}),
        EnqueueOptions {
            priority: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Then high priority
    queue::enqueue(
        &db.pool,
        "app.high",
        &json!({}),
        EnqueueOptions {
            priority: Some(10),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Then medium priority
    queue::enqueue(
        &db.pool,
        "app.medium",
        &json!({}),
        EnqueueOptions {
            priority: Some(5),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Should dequeue highest priority first
    let job1 = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();
    assert_eq!(job1.method, "app.high");

    let job2 = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();
    assert_eq!(job2.method, "app.medium");

    let job3 = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();
    assert_eq!(job3.method, "app.low");
}

#[tokio::test]
async fn test_named_queues_isolation() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    queue::enqueue(
        &db.pool,
        "app.send_email",
        &json!({}),
        EnqueueOptions {
            queue: Some("email".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    queue::enqueue(
        &db.pool,
        "app.process_data",
        &json!({}),
        EnqueueOptions {
            queue: Some("data".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Dequeue from "email" queue — should only get email job
    let email_job = queue::dequeue(&db.pool, "email").await.unwrap().unwrap();
    assert_eq!(email_job.method, "app.send_email");

    // "email" queue should now be empty
    let none = queue::dequeue(&db.pool, "email").await.unwrap();
    assert!(none.is_none());

    // "data" queue should still have its job
    let data_job = queue::dequeue(&db.pool, "data").await.unwrap().unwrap();
    assert_eq!(data_job.method, "app.process_data");
}

#[tokio::test]
async fn test_mark_completed() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    queue::enqueue(&db.pool, "app.complete_me", &json!({}), Default::default())
        .await
        .unwrap();

    let job = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();
    queue::mark_completed(&db.pool, job.id).await.unwrap();

    // Job should not be dequeued again
    let none = queue::dequeue(&db.pool, "default").await.unwrap();
    assert!(none.is_none());

    // Verify status in DB
    let status: String = sqlx::query_scalar("SELECT status FROM \"__job_queue\" WHERE id = $1")
        .bind(job.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(status, "completed");
}

#[tokio::test]
async fn test_mark_failed_with_retry() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    queue::enqueue(
        &db.pool,
        "app.fail_me",
        &json!({}),
        EnqueueOptions {
            max_retries: Some(3),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let job = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();
    assert_eq!(job.attempts, 1);
    assert_eq!(job.max_retries, 3);

    // Mark failed with retry available
    queue::mark_failed(&db.pool, job.id, "Something went wrong", true)
        .await
        .unwrap();

    // Job should be re-queued
    let status: String = sqlx::query_scalar("SELECT status FROM \"__job_queue\" WHERE id = $1")
        .bind(job.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(status, "queued");

    // Can dequeue again
    let job2 = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();
    assert_eq!(job2.id, job.id);
    assert_eq!(job2.attempts, 2); // Incremented
}

#[tokio::test]
async fn test_mark_failed_no_retry() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    queue::enqueue(
        &db.pool,
        "app.permanent_fail",
        &json!({}),
        Default::default(),
    )
    .await
    .unwrap();

    let job = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();

    queue::mark_failed(&db.pool, job.id, "Fatal error", false)
        .await
        .unwrap();

    let status: String = sqlx::query_scalar("SELECT status FROM \"__job_queue\" WHERE id = $1")
        .bind(job.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");

    // Should not be dequeued again
    let none = queue::dequeue(&db.pool, "default").await.unwrap();
    assert!(none.is_none());
}

#[tokio::test]
async fn test_error_stored_on_failure() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    queue::enqueue(&db.pool, "app.error_check", &json!({}), Default::default())
        .await
        .unwrap();

    let job = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();
    queue::mark_failed(&db.pool, job.id, "Connection timed out", false)
        .await
        .unwrap();

    let error: Option<String> =
        sqlx::query_scalar("SELECT error FROM \"__job_queue\" WHERE id = $1")
            .bind(job.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();

    assert_eq!(error.unwrap(), "Connection timed out");
}

#[tokio::test]
async fn test_custom_max_retries() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    queue::enqueue(
        &db.pool,
        "app.custom_retries",
        &json!({}),
        EnqueueOptions {
            max_retries: Some(5),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let job = queue::dequeue(&db.pool, "default").await.unwrap().unwrap();
    assert_eq!(job.max_retries, 5);
}
