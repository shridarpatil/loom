//! Integration tests for database migrations and system table setup.

use loom_tests::*;
use std::sync::Arc;

use loom_core::db::migrate;
use loom_core::doctype::meta::{DocFieldMeta, FieldType};
use loom_core::doctype::DocTypeRegistry;

#[tokio::test]
async fn test_system_tables_created() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    let system_tables = [
        "__doctype",
        "__naming_series",
        "__script",
        "__user_api_key",
        "__customization",
        "__user",
        "__session",
        "__site_config",
        "__user_settings",
        "__user_permission",
        "__job_queue",
        "__activity",
    ];

    for table in &system_tables {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
        )
        .bind(table)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(exists, "System table '{}' should exist", table);
    }
}

#[tokio::test]
async fn test_admin_user_seeded() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    let row: Option<(String, String)> =
        sqlx::query_as("SELECT email, full_name FROM \"__user\" WHERE email = 'Administrator'")
            .fetch_optional(&db.pool)
            .await
            .unwrap();

    assert!(row.is_some(), "Administrator user should be seeded");
    let (email, name) = row.unwrap();
    assert_eq!(email, "Administrator");
    assert_eq!(name, "Administrator");
}

#[tokio::test]
async fn test_admin_password_verification() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    let hash: String =
        sqlx::query_scalar("SELECT password_hash FROM \"__user\" WHERE email = 'Administrator'")
            .fetch_one(&db.pool)
            .await
            .unwrap();

    assert!(migrate::verify_password("admin", &hash));
    assert!(!migrate::verify_password("wrong", &hash));
}

#[tokio::test]
async fn test_doctype_table_creation() {
    skip_without_db!();
    let meta = test_doctype("Test Item");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let table_name = meta.table_name();
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
    )
    .bind(&table_name)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert!(exists, "Table '{}' should be created", table_name);
}

#[tokio::test]
async fn test_doctype_table_has_standard_fields() {
    skip_without_db!();
    let meta = test_doctype("Test Standard");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let table_name = meta.table_name();
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::TEXT FROM information_schema.columns WHERE table_name = $1",
    )
    .bind(&table_name)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    for expected in &[
        "id",
        "owner",
        "creation",
        "modified",
        "modified_by",
        "docstatus",
    ] {
        assert!(
            columns.contains(&expected.to_string()),
            "Standard field '{}' missing from table",
            expected
        );
    }
}

#[tokio::test]
async fn test_doctype_table_has_custom_fields() {
    skip_without_db!();
    let meta = test_doctype("Test Fields");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let table_name = meta.table_name();
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::TEXT FROM information_schema.columns WHERE table_name = $1",
    )
    .bind(&table_name)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert!(columns.contains(&"title".to_string()));
    assert!(columns.contains(&"description".to_string()));
    assert!(columns.contains(&"status".to_string()));
    assert!(columns.contains(&"priority".to_string()));
}

#[tokio::test]
async fn test_alter_table_adds_new_column() {
    skip_without_db!();

    // Start with a basic DocType
    let mut meta = test_doctype("Test Alter");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Add a new field
    meta.fields.push(DocFieldMeta {
        fieldname: "new_field".into(),
        label: Some("New Field".into()),
        fieldtype: FieldType::Data,
        ..DocFieldMeta::default()
    });

    // Re-register and re-migrate
    registry.register(meta.clone()).await;
    migrate::migrate_doctype(&db.pool, &meta).await.unwrap();

    let table_name = meta.table_name();
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::TEXT FROM information_schema.columns WHERE table_name = $1",
    )
    .bind(&table_name)
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert!(
        columns.contains(&"new_field".to_string()),
        "New column should be added by ALTER TABLE"
    );
}

#[tokio::test]
async fn test_migrate_idempotent() {
    skip_without_db!();
    let db = TestDb::new().await;

    // Run system migrations twice — should not fail
    migrate::migrate_system_tables(&db.pool).await.unwrap();
    migrate::migrate_system_tables(&db.pool).await.unwrap();

    let meta = test_doctype("Test Idempotent");
    let registry = Arc::new(DocTypeRegistry::new());
    registry.register(meta.clone()).await;

    // Migrate DocType twice
    migrate::migrate_doctype(&db.pool, &meta).await.unwrap();
    migrate::migrate_doctype(&db.pool, &meta).await.unwrap();
}

#[tokio::test]
async fn test_registry_sync_to_database() {
    skip_without_db!();
    let meta = test_doctype("Test Sync");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    registry.sync_to_database(&db.pool).await.unwrap();

    let row: Option<(String,)> =
        sqlx::query_as("SELECT name FROM \"__doctype\" WHERE name = 'Test Sync'")
            .fetch_optional(&db.pool)
            .await
            .unwrap();

    assert!(row.is_some(), "DocType should be synced to __doctype table");
}

#[tokio::test]
async fn test_registry_load_from_database() {
    skip_without_db!();
    let meta = test_doctype("Test DB Load");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Sync to DB first
    registry.sync_to_database(&db.pool).await.unwrap();

    // Create a fresh registry and load from DB
    let new_registry = DocTypeRegistry::new();
    let count = new_registry.load_from_database(&db.pool).await.unwrap();
    assert!(count > 0);

    let loaded = new_registry.get_meta("Test DB Load").await;
    assert!(loaded.is_ok());
    assert_eq!(loaded.unwrap().name, "Test DB Load");
}
