//! Integration tests for the app lifecycle: create, install, migrate.

use loom_tests::*;
use serde_json::json;
use std::sync::Arc;

use loom_core::db::migrate;
use loom_core::doctype::DocTypeRegistry;

#[tokio::test]
async fn test_create_app_scaffold() {
    skip_without_db!();

    // Create a temp directory for the app
    let temp_dir = std::env::temp_dir().join(format!("loom_test_app_{}", uuid::Uuid::new_v4()));
    let app_dir = temp_dir.join("apps").join("test_hr");

    // Simulate app scaffolding
    std::fs::create_dir_all(app_dir.join("doctypes").join("employee")).unwrap();
    std::fs::create_dir_all(app_dir.join("api")).unwrap();
    std::fs::create_dir_all(app_dir.join("fixtures")).unwrap();

    // Write loom_app.toml
    std::fs::write(
        app_dir.join("loom_app.toml"),
        r#"[app]
name = "test_hr"
version = "1.0.0"
title = "Test HR"
modules = ["HR"]
"#,
    )
    .unwrap();

    // Write a DocType JSON
    let employee_meta = json!({
        "name": "Employee",
        "module": "HR",
        "naming_rule": "hash",
        "fields": [
            {
                "fieldname": "employee_name",
                "label": "Employee Name",
                "fieldtype": "Data",
                "reqd": true,
                "in_list_view": true
            },
            {
                "fieldname": "department",
                "label": "Department",
                "fieldtype": "Data"
            },
            {
                "fieldname": "status",
                "label": "Status",
                "fieldtype": "Select",
                "options": "Active\nInactive"
            }
        ],
        "permissions": [
            { "role": "Administrator", "read": true, "write": true, "create": true, "delete": true }
        ]
    });

    std::fs::write(
        app_dir
            .join("doctypes")
            .join("employee")
            .join("employee.json"),
        serde_json::to_string_pretty(&employee_meta).unwrap(),
    )
    .unwrap();

    // Write a fixture
    let fixture = json!([
        { "employee_name": "John Doe", "department": "Engineering", "status": "Active" },
        { "employee_name": "Jane Smith", "department": "HR", "status": "Active" }
    ]);
    std::fs::write(
        app_dir.join("fixtures").join("employee.json"),
        serde_json::to_string_pretty(&fixture).unwrap(),
    )
    .unwrap();

    // Verify all files exist
    assert!(app_dir.join("loom_app.toml").exists());
    assert!(app_dir.join("doctypes/employee/employee.json").exists());
    assert!(app_dir.join("fixtures/employee.json").exists());

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_register_doctype_from_json_and_migrate() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    // Create a temp directory with a DocType JSON
    let temp_dir = std::env::temp_dir().join(format!("loom_test_dt_{}", uuid::Uuid::new_v4()));
    let dt_dir = temp_dir.join("task");
    std::fs::create_dir_all(&dt_dir).unwrap();

    let dt_json = json!({
        "name": "Task",
        "module": "Test",
        "naming_rule": "hash",
        "fields": [
            { "fieldname": "subject", "label": "Subject", "fieldtype": "Data", "reqd": true },
            { "fieldname": "priority", "label": "Priority", "fieldtype": "Select", "options": "Low\nMedium\nHigh" },
            { "fieldname": "due_date", "label": "Due Date", "fieldtype": "Date" }
        ],
        "permissions": [
            { "role": "Administrator", "read": true, "write": true, "create": true, "delete": true }
        ]
    });

    std::fs::write(
        dt_dir.join("task.json"),
        serde_json::to_string_pretty(&dt_json).unwrap(),
    )
    .unwrap();

    // Load from directory
    let registry = DocTypeRegistry::new();
    let count = registry.load_from_directory(&temp_dir).await.unwrap();
    assert_eq!(count, 1);

    // Migrate
    let meta = registry.get_meta("Task").await.unwrap();
    migrate::migrate_doctype(&db.pool, &meta).await.unwrap();

    // Verify table exists
    let table_name = meta.table_name();
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)",
    )
    .bind(&table_name)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(exists);

    // Insert a document into the migrated table
    let ctx = system_ctx(&db.pool, Arc::new(registry));
    let mut doc = json!({ "subject": "Test Task", "priority": "High" });
    let result = loom_core::doctype::controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();

    assert_eq!(
        result.get("subject").and_then(|v| v.as_str()),
        Some("Test Task")
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_full_app_install_flow() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    // Simulate a complete app directory
    let temp_dir = std::env::temp_dir().join(format!("loom_install_{}", uuid::Uuid::new_v4()));
    let app_dir = temp_dir.join("test_crm");
    let dt_dir = app_dir.join("doctypes").join("contact");
    std::fs::create_dir_all(&dt_dir).unwrap();

    // App manifest
    std::fs::write(
        app_dir.join("loom_app.toml"),
        "[app]\nname = \"test_crm\"\nversion = \"1.0.0\"\ntitle = \"Test CRM\"\nmodules = [\"CRM\"]\n",
    ).unwrap();

    // DocType
    let contact_meta = json!({
        "name": "Contact",
        "module": "CRM",
        "naming_rule": "hash",
        "fields": [
            { "fieldname": "first_name", "label": "First Name", "fieldtype": "Data", "reqd": true },
            { "fieldname": "last_name", "label": "Last Name", "fieldtype": "Data" },
            { "fieldname": "email", "label": "Email", "fieldtype": "Data" },
            { "fieldname": "phone", "label": "Phone", "fieldtype": "Data" }
        ],
        "permissions": [
            { "role": "Administrator", "read": true, "write": true, "create": true, "delete": true }
        ]
    });

    std::fs::write(
        dt_dir.join("contact.json"),
        serde_json::to_string_pretty(&contact_meta).unwrap(),
    )
    .unwrap();

    // Write a Rhai script for the Contact DocType
    std::fs::write(
        dt_dir.join("contact.rhai"),
        "fn validate(doc) {\n  // validation placeholder\n}\n",
    )
    .unwrap();

    // Step 1: Load DocTypes from directory
    let registry = Arc::new(DocTypeRegistry::new());
    let count = registry
        .load_from_directory(&app_dir.join("doctypes"))
        .await
        .unwrap();
    assert_eq!(count, 1);

    // Step 2: Migrate
    let meta = registry.get_meta("Contact").await.unwrap();
    migrate::migrate_doctype(&db.pool, &meta).await.unwrap();

    // Step 3: Sync registry to DB
    registry.sync_to_database(&db.pool).await.unwrap();

    // Step 4: Verify we can do CRUD
    let ctx = system_ctx(&db.pool, registry);
    let mut doc = json!({
        "first_name": "Alice",
        "last_name": "Smith",
        "email": "alice@example.com"
    });
    let result = loom_core::doctype::controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();

    let id = result.get("id").and_then(|v| v.as_str()).unwrap();
    let fetched = loom_core::doctype::controller::get(&ctx, &meta, id)
        .await
        .unwrap();

    assert_eq!(
        fetched.get("first_name").and_then(|v| v.as_str()),
        Some("Alice")
    );
    assert_eq!(
        fetched.get("email").and_then(|v| v.as_str()),
        Some("alice@example.com")
    );

    // Step 5: Verify DocType persisted in __doctype table
    let dt_row: Option<(String,)> =
        sqlx::query_as("SELECT name FROM \"__doctype\" WHERE name = 'Contact'")
            .fetch_optional(&db.pool)
            .await
            .unwrap();
    assert!(dt_row.is_some());

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_multiple_doctypes_in_one_app() {
    skip_without_db!();
    let db = TestDb::migrated().await;

    // Create app with multiple DocTypes
    let temp_dir = std::env::temp_dir().join(format!("loom_multi_dt_{}", uuid::Uuid::new_v4()));

    // DocType 1: Project
    let project_dir = temp_dir.join("project");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(
        project_dir.join("project.json"),
        serde_json::to_string_pretty(&json!({
            "name": "Project",
            "module": "PM",
            "naming_rule": "hash",
            "fields": [
                { "fieldname": "project_name", "label": "Project Name", "fieldtype": "Data", "reqd": true }
            ],
            "permissions": [
                { "role": "Administrator", "read": true, "write": true, "create": true, "delete": true }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    // DocType 2: Milestone
    let milestone_dir = temp_dir.join("milestone");
    std::fs::create_dir_all(&milestone_dir).unwrap();
    std::fs::write(
        milestone_dir.join("milestone.json"),
        serde_json::to_string_pretty(&json!({
            "name": "Milestone",
            "module": "PM",
            "naming_rule": "hash",
            "fields": [
                { "fieldname": "milestone_name", "label": "Milestone Name", "fieldtype": "Data", "reqd": true },
                { "fieldname": "project", "label": "Project", "fieldtype": "Link", "options": "Project" }
            ],
            "permissions": [
                { "role": "Administrator", "read": true, "write": true, "create": true, "delete": true }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    // Load both
    let registry = Arc::new(DocTypeRegistry::new());
    let count = registry.load_from_directory(&temp_dir).await.unwrap();
    assert_eq!(count, 2);

    // Migrate all
    migrate::migrate_all(&db.pool, &registry).await.unwrap();

    // Insert into both
    let ctx = system_ctx(&db.pool, registry.clone());

    let project_meta = registry.get_meta("Project").await.unwrap();
    let mut project = json!({ "project_name": "Loom Framework" });
    let project_result =
        loom_core::doctype::controller::insert(&ctx, &project_meta, &mut project, &noop_hooks())
            .await
            .unwrap();
    let project_id = project_result.get("id").and_then(|v| v.as_str()).unwrap();

    let milestone_meta = registry.get_meta("Milestone").await.unwrap();
    let mut milestone = json!({
        "milestone_name": "v1.0 Release",
        "project": project_id
    });
    let milestone_result = loom_core::doctype::controller::insert(
        &ctx,
        &milestone_meta,
        &mut milestone,
        &noop_hooks(),
    )
    .await
    .unwrap();

    assert_eq!(
        milestone_result
            .get("milestone_name")
            .and_then(|v| v.as_str()),
        Some("v1.0 Release")
    );
    assert_eq!(
        milestone_result.get("project").and_then(|v| v.as_str()),
        Some(project_id)
    );

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_link_validation_on_insert() {
    skip_without_db!();
    let parent_meta = test_doctype("Link Parent");
    let child_meta = test_linked_doctype("Link Child", "Link Parent");
    let (db, registry) = TestDb::with_doctypes(vec![parent_meta.clone(), child_meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    // Try to insert child with non-existent parent link — should fail
    let mut child = json!({
        "title": "Orphan Child",
        "linked_item": "non-existent-parent"
    });
    let result =
        loom_core::doctype::controller::insert(&ctx, &child_meta, &mut child, &noop_hooks()).await;

    assert!(result.is_err(), "Insert with broken link should fail");
}
