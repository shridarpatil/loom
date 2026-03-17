//! Integration tests for permission enforcement with a real database.

use loom_tests::*;
use serde_json::json;

use loom_core::doctype::controller;
use loom_core::doctype::meta::{DocFieldMeta, DocPermMeta, FieldType, Meta};

/// Build a DocType with role-based permissions for testing.
fn multi_role_doctype() -> Meta {
    Meta {
        name: "Perm Doc".to_string(),
        module: "Test".to_string(),
        naming_rule: loom_core::doctype::NamingRule::Hash,
        fields: vec![
            DocFieldMeta {
                fieldname: "title".into(),
                label: Some("Title".into()),
                fieldtype: FieldType::Data,
                reqd: true,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "confidential".into(),
                label: Some("Confidential".into()),
                fieldtype: FieldType::Data,
                permlevel: 1,
                ..DocFieldMeta::default()
            },
        ],
        permissions: vec![
            DocPermMeta {
                role: "Test Reader".into(),
                read: true,
                write: false,
                create: false,
                delete: false,
                permlevel: 0,
                ..DocPermMeta::default()
            },
            DocPermMeta {
                role: "Test Writer".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                permlevel: 0,
                ..DocPermMeta::default()
            },
            DocPermMeta {
                role: "Test Manager".into(),
                read: true,
                write: true,
                create: false,
                delete: false,
                permlevel: 1,
                ..DocPermMeta::default()
            },
        ],
        ..Meta::default()
    }
}

#[tokio::test]
async fn test_admin_bypasses_permissions() {
    skip_without_db!();
    let meta = multi_role_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    // Admin can always create
    let mut doc = json!({ "title": "Admin Doc", "confidential": "secret" });
    let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_user_without_create_perm_denied() {
    skip_without_db!();
    let meta = multi_role_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // User with only "Test Reader" role — no create permission
    let ctx = user_ctx(
        &db.pool,
        registry,
        "reader@test.com",
        vec!["Test Reader".into(), "All".into()],
    );

    let mut doc = json!({ "title": "Reader Doc" });
    let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks()).await;
    assert!(result.is_err(), "Reader should not be able to create");
}

#[tokio::test]
async fn test_user_with_create_perm_allowed() {
    skip_without_db!();
    let meta = multi_role_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // User with "Test Writer" role — has create permission
    let ctx = user_ctx(
        &db.pool,
        registry,
        "writer@test.com",
        vec!["Test Writer".into(), "All".into()],
    );

    let mut doc = json!({ "title": "Writer Doc" });
    let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks()).await;
    assert!(result.is_ok(), "Writer should be able to create");
}

#[tokio::test]
async fn test_read_permission_enforced() {
    skip_without_db!();
    let meta = multi_role_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Insert as admin
    let admin_ctx = system_ctx(&db.pool, registry.clone());
    let mut doc = json!({ "title": "Read Test" });
    let inserted = controller::insert(&admin_ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Reader can read
    let reader_ctx = user_ctx(
        &db.pool,
        registry.clone(),
        "reader@test.com",
        vec!["Test Reader".into(), "All".into()],
    );
    let result = controller::get(&reader_ctx, &meta, id).await;
    assert!(result.is_ok());

    // User with no matching role cannot read
    let nobody_ctx = user_ctx(
        &db.pool,
        registry,
        "nobody@test.com",
        vec!["Random Role".into()],
    );
    let result = controller::get(&nobody_ctx, &meta, id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_write_permission_enforced() {
    skip_without_db!();
    let meta = multi_role_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Insert as admin
    let admin_ctx = system_ctx(&db.pool, registry.clone());
    let mut doc = json!({ "title": "Write Test" });
    let inserted = controller::insert(&admin_ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Reader cannot write
    let reader_ctx = user_ctx(
        &db.pool,
        registry.clone(),
        "reader@test.com",
        vec!["Test Reader".into(), "All".into()],
    );
    let mut update = json!({ "title": "Hacked" });
    let result = controller::update(&reader_ctx, &meta, id, &mut update, &noop_hooks()).await;
    assert!(result.is_err(), "Reader should not be able to write");

    // Writer can write
    let writer_ctx = user_ctx(
        &db.pool,
        registry,
        "writer@test.com",
        vec!["Test Writer".into(), "All".into()],
    );
    let mut update2 = json!({ "title": "Updated By Writer" });
    let result = controller::update(&writer_ctx, &meta, id, &mut update2, &noop_hooks()).await;
    assert!(result.is_ok(), "Writer should be able to write");
}

#[tokio::test]
async fn test_permlevel_field_stripping_on_read() {
    skip_without_db!();
    let meta = multi_role_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Insert as admin with confidential data
    let admin_ctx = system_ctx(&db.pool, registry.clone());
    let mut doc = json!({ "title": "Permlevel Test", "confidential": "top-secret" });
    let inserted = controller::insert(&admin_ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Reader (permlevel 0 only) should not see confidential field
    let reader_ctx = user_ctx(
        &db.pool,
        registry.clone(),
        "reader@test.com",
        vec!["Test Reader".into(), "All".into()],
    );
    let fetched = controller::get(&reader_ctx, &meta, id).await.unwrap();
    assert!(
        fetched.get("confidential").is_none(),
        "Confidential field should be stripped for reader"
    );
    assert!(
        fetched.get("title").is_some(),
        "Title should still be visible"
    );

    // Manager (permlevel 0 + 1) should see confidential field
    let manager_ctx = user_ctx(
        &db.pool,
        registry,
        "manager@test.com",
        vec!["Test Reader".into(), "Test Manager".into(), "All".into()],
    );
    let fetched = controller::get(&manager_ctx, &meta, id).await.unwrap();
    assert_eq!(
        fetched
            .get("confidential")
            .and_then(|v| v.as_str())
            .unwrap(),
        "top-secret"
    );
}

#[tokio::test]
async fn test_permlevel_write_enforcement() {
    skip_without_db!();
    let meta = multi_role_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Insert as admin
    let admin_ctx = system_ctx(&db.pool, registry.clone());
    let mut doc = json!({ "title": "Write Level Test", "confidential": "initial" });
    let inserted = controller::insert(&admin_ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Writer (has write at level 0 but NOT level 1) tries to update confidential field
    let writer_ctx = user_ctx(
        &db.pool,
        registry.clone(),
        "writer@test.com",
        vec!["Test Writer".into(), "All".into()],
    );
    let mut update = json!({ "confidential": "hacked" });
    let result = controller::update(&writer_ctx, &meta, id, &mut update, &noop_hooks()).await;
    assert!(
        result.is_err(),
        "Writer should not be able to update permlevel 1 field"
    );

    // Manager (has write at level 1) can update confidential field
    let manager_ctx = user_ctx(
        &db.pool,
        registry,
        "manager@test.com",
        vec![
            "Test Reader".into(),
            "Test Writer".into(),
            "Test Manager".into(),
            "All".into(),
        ],
    );
    let mut update2 = json!({ "confidential": "updated-by-manager" });
    let result = controller::update(&manager_ctx, &meta, id, &mut update2, &noop_hooks()).await;
    assert!(
        result.is_ok(),
        "Manager should be able to update permlevel 1 field"
    );
}

#[tokio::test]
async fn test_if_owner_permission() {
    skip_without_db!();
    let meta = Meta {
        name: "Owner Doc".to_string(),
        module: "Test".to_string(),
        naming_rule: loom_core::doctype::NamingRule::Hash,
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            label: Some("Title".into()),
            fieldtype: FieldType::Data,
            reqd: true,
            ..DocFieldMeta::default()
        }],
        permissions: vec![DocPermMeta {
            role: "Test Owner".into(),
            read: true,
            write: true,
            create: true,
            delete: false,
            if_owner: true,
            permlevel: 0,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    };

    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Insert as user A
    let user_a_ctx = user_ctx(
        &db.pool,
        registry.clone(),
        "user_a@test.com",
        vec!["Test Owner".into(), "All".into()],
    );
    let mut doc = json!({ "title": "User A's Doc" });
    let inserted = controller::insert(&user_a_ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // User A can read their own doc
    let result = controller::get(&user_a_ctx, &meta, id).await;
    assert!(result.is_ok(), "Owner should be able to read their own doc");

    // User B cannot read user A's doc (if_owner enforced)
    let user_b_ctx = user_ctx(
        &db.pool,
        registry,
        "user_b@test.com",
        vec!["Test Owner".into(), "All".into()],
    );
    let result = controller::get(&user_b_ctx, &meta, id).await;
    assert!(result.is_err(), "Non-owner should not be able to read doc");
}

#[tokio::test]
async fn test_delete_permission_enforced() {
    skip_without_db!();
    let meta = multi_role_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Insert as admin
    let admin_ctx = system_ctx(&db.pool, registry.clone());
    let mut doc = json!({ "title": "Delete Test" });
    let inserted = controller::insert(&admin_ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Reader cannot delete
    let reader_ctx = user_ctx(
        &db.pool,
        registry.clone(),
        "reader@test.com",
        vec!["Test Reader".into(), "All".into()],
    );
    let result = controller::delete(&reader_ctx, &meta, id, &noop_hooks()).await;
    assert!(result.is_err(), "Reader should not be able to delete");

    // Writer can delete
    let writer_ctx = user_ctx(
        &db.pool,
        registry,
        "writer@test.com",
        vec!["Test Writer".into(), "All".into()],
    );
    let result = controller::delete(&writer_ctx, &meta, id, &noop_hooks()).await;
    assert!(result.is_ok(), "Writer should be able to delete");
}
