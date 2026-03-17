//! Integration tests for User Permission (row-level filtering) with a real database.

use loom_tests::*;
use serde_json::json;

use loom_core::doctype::controller;
use loom_core::doctype::meta::{DocFieldMeta, DocPermMeta, FieldType, Meta, NamingRule};

/// A DocType with a Link field to test user-permission-based filtering.
fn company_doctype() -> Meta {
    Meta {
        name: "Test Company".into(),
        module: "Test".into(),
        naming_rule: NamingRule::Hash,
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            label: Some("Title".into()),
            fieldtype: FieldType::Data,
            reqd: true,
            in_list_view: true,
            ..DocFieldMeta::default()
        }],
        permissions: vec![
            DocPermMeta {
                role: "Administrator".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                ..DocPermMeta::default()
            },
            DocPermMeta {
                role: "Test User".into(),
                read: true,
                write: true,
                create: true,
                delete: false,
                ..DocPermMeta::default()
            },
        ],
        ..Meta::default()
    }
}

/// A DocType with a Link field pointing to "Test Company".
fn employee_doctype() -> Meta {
    Meta {
        name: "Test Employee".into(),
        module: "Test".into(),
        naming_rule: NamingRule::Hash,
        fields: vec![
            DocFieldMeta {
                fieldname: "title".into(),
                label: Some("Title".into()),
                fieldtype: FieldType::Data,
                reqd: true,
                in_list_view: true,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "company".into(),
                label: Some("Company".into()),
                fieldtype: FieldType::Link,
                options: Some("Test Company".into()),
                ..DocFieldMeta::default()
            },
        ],
        permissions: vec![
            DocPermMeta {
                role: "Administrator".into(),
                read: true,
                write: true,
                create: true,
                delete: true,
                ..DocPermMeta::default()
            },
            DocPermMeta {
                role: "Test User".into(),
                read: true,
                write: true,
                create: true,
                delete: false,
                ..DocPermMeta::default()
            },
        ],
        ..Meta::default()
    }
}

#[tokio::test]
async fn test_user_perm_restricts_list() {
    skip_without_db!();
    let company_meta = company_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![company_meta.clone()]).await;
    let admin_ctx = system_ctx(&db.pool, registry.clone());

    // Insert two companies as admin
    let mut c1 = json!({ "title": "Acme Corp" });
    let c1_result = controller::insert(&admin_ctx, &company_meta, &mut c1, &noop_hooks())
        .await
        .unwrap();
    let c1_id = c1_result.get("id").and_then(|v| v.as_str()).unwrap();

    let mut c2 = json!({ "title": "Globex Inc" });
    controller::insert(&admin_ctx, &company_meta, &mut c2, &noop_hooks())
        .await
        .unwrap();

    // Insert a UserPermission restricting user to only see Acme Corp
    sqlx::query(
        "INSERT INTO \"__user_permission\" (user_email, allow, for_value, applicable_for) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind("user@test.com")
    .bind("Test Company")
    .bind(c1_id)
    .bind(None::<String>)
    .execute(&db.pool)
    .await
    .unwrap();

    // User context with Test User role
    let user_ctx = user_ctx(
        &db.pool,
        registry,
        "user@test.com",
        vec!["Test User".into(), "All".into()],
    );

    // get_list should only return the company the user has permission for
    let list = controller::get_list(
        &user_ctx,
        &company_meta,
        None,
        None,
        None,
        Some(10),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(list.len(), 1, "User should only see one company");
    assert_eq!(
        list[0].get("id").and_then(|v| v.as_str()).unwrap(),
        c1_id,
        "User should only see Acme Corp"
    );
}

#[tokio::test]
async fn test_user_perm_no_restriction_without_perms() {
    skip_without_db!();
    let company_meta = company_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![company_meta.clone()]).await;
    let admin_ctx = system_ctx(&db.pool, registry.clone());

    // Insert two companies
    let mut c1 = json!({ "title": "Alpha Co" });
    controller::insert(&admin_ctx, &company_meta, &mut c1, &noop_hooks())
        .await
        .unwrap();

    let mut c2 = json!({ "title": "Beta Co" });
    controller::insert(&admin_ctx, &company_meta, &mut c2, &noop_hooks())
        .await
        .unwrap();

    // User with NO UserPermission entries — should see all docs
    let user_ctx = user_ctx(
        &db.pool,
        registry,
        "free_user@test.com",
        vec!["Test User".into(), "All".into()],
    );

    let list = controller::get_list(
        &user_ctx,
        &company_meta,
        None,
        None,
        None,
        Some(10),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        list.len(),
        2,
        "User without UserPermissions should see all docs"
    );
}

#[tokio::test]
async fn test_user_perm_with_link_field() {
    skip_without_db!();
    let company_meta = company_doctype();
    let employee_meta = employee_doctype();
    let (db, registry) =
        TestDb::with_doctypes(vec![company_meta.clone(), employee_meta.clone()]).await;
    let admin_ctx = system_ctx(&db.pool, registry.clone());

    // Insert two companies
    let mut c1 = json!({ "title": "Acme Corp" });
    let c1_result = controller::insert(&admin_ctx, &company_meta, &mut c1, &noop_hooks())
        .await
        .unwrap();
    let c1_id = c1_result.get("id").and_then(|v| v.as_str()).unwrap();

    let mut c2 = json!({ "title": "Globex Inc" });
    let c2_result = controller::insert(&admin_ctx, &company_meta, &mut c2, &noop_hooks())
        .await
        .unwrap();
    let c2_id = c2_result.get("id").and_then(|v| v.as_str()).unwrap();

    // Insert employees in different companies
    let mut e1 = json!({ "title": "Alice", "company": c1_id });
    controller::insert(&admin_ctx, &employee_meta, &mut e1, &noop_hooks())
        .await
        .unwrap();

    let mut e2 = json!({ "title": "Bob", "company": c2_id });
    controller::insert(&admin_ctx, &employee_meta, &mut e2, &noop_hooks())
        .await
        .unwrap();

    let mut e3 = json!({ "title": "Charlie", "company": c1_id });
    controller::insert(&admin_ctx, &employee_meta, &mut e3, &noop_hooks())
        .await
        .unwrap();

    // Restrict user to only Acme Corp via UserPermission on Test Company
    sqlx::query(
        "INSERT INTO \"__user_permission\" (user_email, allow, for_value, applicable_for) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind("linkuser@test.com")
    .bind("Test Company")
    .bind(c1_id)
    .bind(None::<String>)
    .execute(&db.pool)
    .await
    .unwrap();

    let user_ctx = user_ctx(
        &db.pool,
        registry,
        "linkuser@test.com",
        vec!["Test User".into(), "All".into()],
    );

    // Listing employees should only return those linked to Acme Corp
    let list = controller::get_list(
        &user_ctx,
        &employee_meta,
        None,
        None,
        None,
        Some(10),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(list.len(), 2, "User should only see employees in Acme Corp");
    for emp in &list {
        assert_eq!(
            emp.get("company").and_then(|v| v.as_str()).unwrap(),
            c1_id,
            "All returned employees should belong to Acme Corp"
        );
    }
}

#[tokio::test]
async fn test_user_perm_applicable_for() {
    skip_without_db!();
    let company_meta = company_doctype();
    let employee_meta = employee_doctype();
    let (db, registry) =
        TestDb::with_doctypes(vec![company_meta.clone(), employee_meta.clone()]).await;
    let admin_ctx = system_ctx(&db.pool, registry.clone());

    // Insert two companies
    let mut c1 = json!({ "title": "Acme Corp" });
    let c1_result = controller::insert(&admin_ctx, &company_meta, &mut c1, &noop_hooks())
        .await
        .unwrap();
    let c1_id = c1_result.get("id").and_then(|v| v.as_str()).unwrap();

    let mut c2 = json!({ "title": "Globex Inc" });
    let c2_result = controller::insert(&admin_ctx, &company_meta, &mut c2, &noop_hooks())
        .await
        .unwrap();
    let c2_id = c2_result.get("id").and_then(|v| v.as_str()).unwrap();

    // Insert employees in different companies
    let mut e1 = json!({ "title": "Alice", "company": c1_id });
    controller::insert(&admin_ctx, &employee_meta, &mut e1, &noop_hooks())
        .await
        .unwrap();

    let mut e2 = json!({ "title": "Bob", "company": c2_id });
    controller::insert(&admin_ctx, &employee_meta, &mut e2, &noop_hooks())
        .await
        .unwrap();

    // Insert UserPermission with applicable_for = "Test Employee" only
    // This restriction should apply ONLY when listing Test Employee docs,
    // not when listing Test Company docs.
    sqlx::query(
        "INSERT INTO \"__user_permission\" (user_email, allow, for_value, applicable_for) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind("scoped@test.com")
    .bind("Test Company")
    .bind(c1_id)
    .bind(Some("Test Employee"))
    .execute(&db.pool)
    .await
    .unwrap();

    let user_ctx = user_ctx(
        &db.pool,
        registry,
        "scoped@test.com",
        vec!["Test User".into(), "All".into()],
    );

    // Listing companies should NOT be restricted (applicable_for is Test Employee, not Test Company)
    let company_list = controller::get_list(
        &user_ctx,
        &company_meta,
        None,
        None,
        None,
        Some(10),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        company_list.len(),
        2,
        "User should see all companies (applicable_for does not apply to Test Company)"
    );

    // Listing employees SHOULD be restricted to Acme Corp employees
    let employee_list = controller::get_list(
        &user_ctx,
        &employee_meta,
        None,
        None,
        None,
        Some(10),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        employee_list.len(),
        1,
        "User should only see employees in Acme Corp (applicable_for = Test Employee)"
    );
    assert_eq!(
        employee_list[0]
            .get("company")
            .and_then(|v| v.as_str())
            .unwrap(),
        c1_id
    );
}
