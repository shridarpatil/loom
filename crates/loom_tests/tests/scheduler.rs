//! Integration tests for the scheduler's `load_from_apps` functionality.
//!
//! Since `Scheduler.tasks` is private and `Scheduler` does not implement `Debug`,
//! we test `load_from_apps` indirectly through the observable behavior of `run()`:
//! - When no tasks are loaded, `run()` returns immediately.
//! - When tasks are loaded, `run()` enters an infinite loop and enqueues matching
//!   jobs into the `__job_queue` table, which we can query.

use std::sync::Arc;

use loom_tests::*;

use loom_queue::Scheduler;

#[tokio::test]
async fn test_load_from_apps_parses_hooks_toml() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    // Create a temporary apps directory with a valid hooks.toml
    let tmp_dir = std::env::temp_dir().join(format!("loom_test_sched_{}", uuid::Uuid::new_v4()));
    let app_dir = tmp_dir.join("my_test_app");
    std::fs::create_dir_all(&app_dir).unwrap();

    // Use "* * * * *" so the cron matches immediately at any time
    let hooks_toml = r#"
[[scheduler]]
cron = "* * * * *"
method = "api/sync_attendance.rhai"

[[scheduler]]
cron = "* * * * *"
method = "api/check_deadlines.rhai"
"#;
    std::fs::write(app_dir.join("hooks.toml"), hooks_toml).unwrap();

    let pool = Arc::new(db.pool.clone());
    let mut scheduler = Scheduler::new(pool);
    scheduler.load_from_apps(&tmp_dir);

    // If tasks were loaded, run() will try to enqueue them and then sleep.
    // We use a timeout to break out of the infinite loop.
    let run_result = tokio::time::timeout(std::time::Duration::from_secs(3), scheduler.run()).await;

    // run() should NOT have completed (it enters an infinite sleep loop) — so we expect a timeout
    assert!(
        run_result.is_err(),
        "run() should not return immediately when tasks are loaded (it loops)"
    );

    // Verify that the scheduler enqueued jobs into __job_queue
    let methods: Vec<String> = sqlx::query_scalar("SELECT method FROM \"__job_queue\" ORDER BY id")
        .fetch_all(&db.pool)
        .await
        .unwrap();

    assert!(
        methods.contains(&"api/sync_attendance.rhai".to_string()),
        "sync_attendance task should have been enqueued: {:?}",
        methods
    );
    assert!(
        methods.contains(&"api/check_deadlines.rhai".to_string()),
        "check_deadlines task should have been enqueued: {:?}",
        methods
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test]
async fn test_load_from_apps_no_dir() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    let non_existent = std::env::temp_dir().join("loom_no_such_dir_9999999");

    let pool = Arc::new(db.pool.clone());
    let mut scheduler = Scheduler::new(pool);

    // Should not panic when directory does not exist
    scheduler.load_from_apps(&non_existent);

    // run() should return immediately when no tasks are loaded
    let run_result = tokio::time::timeout(std::time::Duration::from_secs(2), scheduler.run()).await;

    assert!(
        run_result.is_ok(),
        "run() should return immediately when no tasks are loaded"
    );
}

#[tokio::test]
async fn test_load_from_apps_no_hooks_file() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    // Create apps directory with an app subdirectory but no hooks.toml
    let tmp_dir = std::env::temp_dir().join(format!("loom_test_nohooks_{}", uuid::Uuid::new_v4()));
    let app_dir = tmp_dir.join("empty_app");
    std::fs::create_dir_all(&app_dir).unwrap();

    // Create some other file (not hooks.toml)
    std::fs::write(app_dir.join("loom_app.toml"), "[app]\nname = \"empty_app\"").unwrap();

    let pool = Arc::new(db.pool.clone());
    let mut scheduler = Scheduler::new(pool);
    scheduler.load_from_apps(&tmp_dir);

    // run() should return immediately (no tasks loaded)
    let run_result = tokio::time::timeout(std::time::Duration::from_secs(2), scheduler.run()).await;

    assert!(
        run_result.is_ok(),
        "run() should return immediately when no hooks.toml is present"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test]
async fn test_load_from_apps_invalid_toml() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    // Create apps directory with a malformed hooks.toml
    let tmp_dir = std::env::temp_dir().join(format!("loom_test_badtoml_{}", uuid::Uuid::new_v4()));
    let app_dir = tmp_dir.join("bad_app");
    std::fs::create_dir_all(&app_dir).unwrap();

    // Write invalid TOML content
    std::fs::write(app_dir.join("hooks.toml"), "this is not {{ valid toml ]]").unwrap();

    let pool = Arc::new(db.pool.clone());
    let mut scheduler = Scheduler::new(pool);

    // Should not panic — invalid TOML is skipped gracefully
    scheduler.load_from_apps(&tmp_dir);

    // run() should return immediately (no valid tasks loaded)
    let run_result = tokio::time::timeout(std::time::Duration::from_secs(2), scheduler.run()).await;

    assert!(
        run_result.is_ok(),
        "run() should return immediately when hooks.toml is invalid"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
