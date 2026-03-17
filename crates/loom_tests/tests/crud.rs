//! Integration tests for CRUD operations with a real database.

use loom_tests::*;
use serde_json::json;

use loom_core::doctype::crud;

#[tokio::test]
async fn test_insert_and_get() {
    skip_without_db!();
    let meta = test_doctype("Crud Item");
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({
        "title": "Test Item 1",
        "description": "A test item",
        "status": "Open"
    });

    let result = crud::insert_doc(ctx.pool(), &meta, &mut doc, "Administrator")
        .await
        .unwrap();

    let id = result.get("id").and_then(|v| v.as_str()).unwrap();
    assert!(!id.is_empty());

    // Fetch it back
    let fetched = crud::get_doc(ctx.pool(), &meta, id).await.unwrap();
    assert_eq!(
        fetched.get("title").and_then(|v| v.as_str()).unwrap(),
        "Test Item 1"
    );
    assert_eq!(
        fetched.get("description").and_then(|v| v.as_str()).unwrap(),
        "A test item"
    );
}

#[tokio::test]
async fn test_insert_sets_standard_fields() {
    skip_without_db!();
    let meta = test_doctype("Crud Standard");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let mut doc = json!({ "title": "Check Standard Fields" });
    let result = crud::insert_doc(&db.pool, &meta, &mut doc, "Administrator")
        .await
        .unwrap();

    assert_eq!(
        result.get("owner").and_then(|v| v.as_str()).unwrap(),
        "Administrator"
    );
    assert!(result.get("creation").is_some());
    assert!(result.get("modified").is_some());
    assert_eq!(
        result.get("modified_by").and_then(|v| v.as_str()).unwrap(),
        "Administrator"
    );
    assert_eq!(
        result
            .get("docstatus")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1),
        0
    );
}

#[tokio::test]
async fn test_insert_required_field_missing() {
    skip_without_db!();
    let meta = test_doctype("Crud Required");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // "title" is required but missing
    let mut doc = json!({ "description": "No title" });
    let result = crud::insert_doc(&db.pool, &meta, &mut doc, "Administrator").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_doc() {
    skip_without_db!();
    let meta = test_doctype("Crud Update");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let mut doc = json!({ "title": "Original Title" });
    let result = crud::insert_doc(&db.pool, &meta, &mut doc, "Administrator")
        .await
        .unwrap();
    let id = result.get("id").and_then(|v| v.as_str()).unwrap();

    let mut update = json!({ "title": "Updated Title" });
    let updated = crud::update_doc(&db.pool, &meta, id, &mut update, "Administrator")
        .await
        .unwrap();

    assert_eq!(
        updated.get("title").and_then(|v| v.as_str()).unwrap(),
        "Updated Title"
    );
}

#[tokio::test]
async fn test_update_nonexistent_doc() {
    skip_without_db!();
    let meta = test_doctype("Crud Update NE");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let mut update = json!({ "title": "Ghost" });
    let result = crud::update_doc(
        &db.pool,
        &meta,
        "nonexistent-id",
        &mut update,
        "Administrator",
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_doc() {
    skip_without_db!();
    let meta = test_doctype("Crud Delete");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let mut doc = json!({ "title": "To Be Deleted" });
    let result = crud::insert_doc(&db.pool, &meta, &mut doc, "Administrator")
        .await
        .unwrap();
    let id = result.get("id").and_then(|v| v.as_str()).unwrap();

    crud::delete_doc(&db.pool, &meta, id).await.unwrap();

    let fetched = crud::get_doc(&db.pool, &meta, id).await;
    assert!(fetched.is_err()); // Should be NotFound
}

#[tokio::test]
async fn test_delete_nonexistent_doc() {
    skip_without_db!();
    let meta = test_doctype("Crud Del NE");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let result = crud::delete_doc(&db.pool, &meta, "does-not-exist").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_list_basic() {
    skip_without_db!();
    let meta = test_doctype("Crud List");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Insert 3 items
    for i in 1..=3 {
        let mut doc = json!({ "title": format!("Item {}", i), "status": "Open" });
        crud::insert_doc(&db.pool, &meta, &mut doc, "Administrator")
            .await
            .unwrap();
    }

    let list = crud::get_list(&db.pool, &meta, None, None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(list.len(), 3);
}

#[tokio::test]
async fn test_get_list_with_filters() {
    skip_without_db!();
    let meta = test_doctype("Crud Filter");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let mut doc1 = json!({ "title": "Open Item", "status": "Open" });
    let mut doc2 = json!({ "title": "Closed Item", "status": "Closed" });
    crud::insert_doc(&db.pool, &meta, &mut doc1, "Administrator")
        .await
        .unwrap();
    crud::insert_doc(&db.pool, &meta, &mut doc2, "Administrator")
        .await
        .unwrap();

    // Object-style filter
    let filters = json!({ "status": "Open" });
    let list = crud::get_list(
        &db.pool,
        &meta,
        Some(&filters),
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].get("title").and_then(|v| v.as_str()).unwrap(),
        "Open Item"
    );
}

#[tokio::test]
async fn test_get_list_array_filters() {
    skip_without_db!();
    let meta = test_doctype("Crud ArrFilter");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let mut doc1 = json!({ "title": "Alpha", "status": "Open" });
    let mut doc2 = json!({ "title": "Beta", "status": "Closed" });
    let mut doc3 = json!({ "title": "Gamma", "status": "Open" });
    crud::insert_doc(&db.pool, &meta, &mut doc1, "Administrator")
        .await
        .unwrap();
    crud::insert_doc(&db.pool, &meta, &mut doc2, "Administrator")
        .await
        .unwrap();
    crud::insert_doc(&db.pool, &meta, &mut doc3, "Administrator")
        .await
        .unwrap();

    // Array-style filter with operator
    let filters = json!([["status", "=", "Open"]]);
    let list = crud::get_list(
        &db.pool,
        &meta,
        Some(&filters),
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_get_list_pagination() {
    skip_without_db!();
    let meta = test_doctype("Crud Page");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    for i in 1..=5 {
        let mut doc = json!({ "title": format!("Page Item {}", i) });
        crud::insert_doc(&db.pool, &meta, &mut doc, "Administrator")
            .await
            .unwrap();
    }

    // Get first 2
    let page1 = crud::get_list(
        &db.pool,
        &meta,
        None,
        None,
        Some("id ASC"),
        Some(2),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(page1.len(), 2);

    // Get next 2 with offset
    let page2 = crud::get_list(
        &db.pool,
        &meta,
        None,
        None,
        Some("id ASC"),
        Some(2),
        Some(2),
        None,
    )
    .await
    .unwrap();
    assert_eq!(page2.len(), 2);

    // Pages should be different
    assert_ne!(
        page1[0].get("id").and_then(|v| v.as_str()),
        page2[0].get("id").and_then(|v| v.as_str())
    );
}

#[tokio::test]
async fn test_get_list_search() {
    skip_without_db!();
    let meta = test_doctype("Crud Search");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let mut doc1 = json!({ "title": "Apple Pie" });
    let mut doc2 = json!({ "title": "Banana Split" });
    crud::insert_doc(&db.pool, &meta, &mut doc1, "Administrator")
        .await
        .unwrap();
    crud::insert_doc(&db.pool, &meta, &mut doc2, "Administrator")
        .await
        .unwrap();

    let list = crud::get_list(&db.pool, &meta, None, None, None, None, None, Some("Apple"))
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].get("title").and_then(|v| v.as_str()).unwrap(),
        "Apple Pie"
    );
}

#[tokio::test]
async fn test_get_count() {
    skip_without_db!();
    let meta = test_doctype("Crud Count");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    for i in 1..=4 {
        let mut doc = json!({ "title": format!("Count Item {}", i), "status": if i % 2 == 0 { "Closed" } else { "Open" } });
        crud::insert_doc(&db.pool, &meta, &mut doc, "Administrator")
            .await
            .unwrap();
    }

    let total = crud::get_count(&db.pool, &meta, None).await.unwrap();
    assert_eq!(total, 4);

    let open_count = crud::get_count(&db.pool, &meta, Some(&json!({ "status": "Open" })))
        .await
        .unwrap();
    assert_eq!(open_count, 2);
}

#[tokio::test]
async fn test_exists() {
    skip_without_db!();
    let meta = test_doctype("Crud Exists");
    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    let mut doc = json!({ "title": "Exists Check", "status": "Open" });
    crud::insert_doc(&db.pool, &meta, &mut doc, "Administrator")
        .await
        .unwrap();

    let exists = crud::exists(&db.pool, &meta, &json!({ "status": "Open" }))
        .await
        .unwrap();
    assert!(exists);

    let not_exists = crud::exists(&db.pool, &meta, &json!({ "status": "Cancelled" }))
        .await
        .unwrap();
    assert!(!not_exists);
}
