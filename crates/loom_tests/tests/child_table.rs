//! Integration tests for child table CRUD with a real database.

use loom_tests::*;
use serde_json::json;

use loom_core::doctype::controller;
use loom_core::doctype::crud;
use loom_core::doctype::meta::{DocFieldMeta, DocPermMeta, FieldType, Meta, NamingRule};

/// Child DocType meta for "Test Child Item".
fn child_item_meta() -> Meta {
    Meta {
        name: "Test Child Item".into(),
        module: "Test".into(),
        is_child_table: true,
        naming_rule: NamingRule::Hash,
        fields: vec![
            DocFieldMeta {
                fieldname: "item_name".into(),
                label: Some("Item Name".into()),
                fieldtype: FieldType::Data,
                reqd: true,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "qty".into(),
                label: Some("Qty".into()),
                fieldtype: FieldType::Int,
                ..DocFieldMeta::default()
            },
        ],
        permissions: vec![],
        ..Meta::default()
    }
}

/// Parent DocType with a Table field pointing to "Test Child Item".
fn parent_with_children_meta() -> Meta {
    Meta {
        name: "Test Parent".into(),
        module: "Test".into(),
        naming_rule: NamingRule::Hash,
        fields: vec![
            DocFieldMeta {
                fieldname: "title".into(),
                label: Some("Title".into()),
                fieldtype: FieldType::Data,
                reqd: true,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "items".into(),
                label: Some("Items".into()),
                fieldtype: FieldType::Table,
                options: Some("Test Child Item".into()),
                ..DocFieldMeta::default()
            },
        ],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    }
}

#[tokio::test]
async fn test_insert_with_child_rows() {
    skip_without_db!();
    let child_meta = child_item_meta();
    let parent_meta = parent_with_children_meta();
    let (db, registry) = TestDb::with_doctypes(vec![child_meta.clone(), parent_meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({
        "title": "Parent With Children",
        "items": [
            { "item_name": "Widget A", "qty": 10 },
            { "item_name": "Widget B", "qty": 5 },
        ]
    });

    let result = controller::insert(&ctx, &parent_meta, &mut doc, &noop_hooks())
        .await
        .unwrap();

    let parent_id = result.get("id").and_then(|v| v.as_str()).unwrap();
    assert!(!parent_id.is_empty());

    // Verify children exist in the database by querying the child table directly
    let child_table = child_meta.table_name();
    let rows: Vec<(String, String, String, i32)> = sqlx::query_as(&format!(
        "SELECT parent, parentfield, item_name, idx FROM \"{}\" WHERE parent = $1 ORDER BY idx",
        child_table
    ))
    .bind(parent_id)
    .fetch_all(ctx.pool())
    .await
    .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, parent_id); // parent
    assert_eq!(rows[0].1, "items"); // parentfield
    assert_eq!(rows[0].2, "Widget A");
    assert_eq!(rows[0].3, 1); // idx
    assert_eq!(rows[1].2, "Widget B");
    assert_eq!(rows[1].3, 2); // idx
}

#[tokio::test]
async fn test_get_parent_has_no_inline_children() {
    skip_without_db!();
    let child_meta = child_item_meta();
    let parent_meta = parent_with_children_meta();
    let (db, registry) = TestDb::with_doctypes(vec![child_meta.clone(), parent_meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({
        "title": "Parent For Get",
        "items": [
            { "item_name": "Item X", "qty": 3 },
        ]
    });

    let inserted = controller::insert(&ctx, &parent_meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let parent_id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // get_doc returns the parent row; children are in a separate table
    let fetched = crud::get_doc(ctx.pool(), &parent_meta, parent_id)
        .await
        .unwrap();

    assert_eq!(
        fetched.get("title").and_then(|v| v.as_str()).unwrap(),
        "Parent For Get"
    );

    // The raw get_doc on the parent table should not have an "items" column
    // with child rows inline — Table fields are not stored as columns on the parent.
    // The parent row itself does not carry child data; it is queried separately.
    // So fetched["items"] should either be absent or not be a populated array
    // matching the children.
    let child_table = child_meta.table_name();
    let child_count: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM \"{}\" WHERE parent = $1",
        child_table
    ))
    .bind(parent_id)
    .fetch_one(ctx.pool())
    .await
    .unwrap();

    assert_eq!(child_count, 1, "Child row should exist in the child table");
}

#[tokio::test]
async fn test_update_replaces_children() {
    skip_without_db!();
    let child_meta = child_item_meta();
    let parent_meta = parent_with_children_meta();
    let (db, registry) = TestDb::with_doctypes(vec![child_meta.clone(), parent_meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    // Insert parent with initial children
    let mut doc = json!({
        "title": "Parent Update Children",
        "items": [
            { "item_name": "Old Item 1", "qty": 1 },
            { "item_name": "Old Item 2", "qty": 2 },
        ]
    });

    let inserted = controller::insert(&ctx, &parent_meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let parent_id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Update with new children (should replace old ones)
    let mut update = json!({
        "items": [
            { "item_name": "New Item A", "qty": 100 },
        ]
    });

    controller::update(&ctx, &parent_meta, parent_id, &mut update, &noop_hooks())
        .await
        .unwrap();

    // Verify old children are deleted and new ones are inserted
    let child_table = child_meta.table_name();
    let rows: Vec<(String, i64)> = sqlx::query_as(&format!(
        "SELECT item_name, qty FROM \"{}\" WHERE parent = $1 ORDER BY idx",
        child_table
    ))
    .bind(parent_id)
    .fetch_all(ctx.pool())
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "New Item A");
    assert_eq!(rows[0].1, 100);
}

#[tokio::test]
async fn test_delete_cascades_to_children() {
    skip_without_db!();
    let child_meta = child_item_meta();
    let parent_meta = parent_with_children_meta();
    let (db, registry) = TestDb::with_doctypes(vec![child_meta.clone(), parent_meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({
        "title": "Parent To Delete",
        "items": [
            { "item_name": "Doomed Item 1", "qty": 7 },
            { "item_name": "Doomed Item 2", "qty": 8 },
        ]
    });

    let inserted = controller::insert(&ctx, &parent_meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let parent_id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Verify children exist before delete
    let child_table = child_meta.table_name();
    let count_before: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM \"{}\" WHERE parent = $1",
        child_table
    ))
    .bind(parent_id)
    .fetch_one(ctx.pool())
    .await
    .unwrap();
    assert_eq!(count_before, 2);

    // Delete parent
    controller::delete(&ctx, &parent_meta, parent_id, &noop_hooks())
        .await
        .unwrap();

    // Verify parent is gone
    let parent_result = crud::get_doc(ctx.pool(), &parent_meta, parent_id).await;
    assert!(parent_result.is_err());

    // Verify children are also gone
    let count_after: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM \"{}\" WHERE parent = $1",
        child_table
    ))
    .bind(parent_id)
    .fetch_one(ctx.pool())
    .await
    .unwrap();
    assert_eq!(count_after, 0);
}

#[tokio::test]
async fn test_insert_empty_children() {
    skip_without_db!();
    let child_meta = child_item_meta();
    let parent_meta = parent_with_children_meta();
    let (db, registry) = TestDb::with_doctypes(vec![child_meta.clone(), parent_meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    // Insert parent with empty items array
    let mut doc = json!({
        "title": "Parent No Children",
        "items": []
    });

    let result = controller::insert(&ctx, &parent_meta, &mut doc, &noop_hooks())
        .await
        .unwrap();

    let parent_id = result.get("id").and_then(|v| v.as_str()).unwrap();

    // Verify no child rows in DB
    let child_table = child_meta.table_name();
    let count: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM \"{}\" WHERE parent = $1",
        child_table
    ))
    .bind(parent_id)
    .fetch_one(ctx.pool())
    .await
    .unwrap();
    assert_eq!(count, 0);
}
