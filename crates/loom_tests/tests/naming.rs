//! Integration tests for DocType naming rules with a real database.
//! Series naming, prompt naming, by_fieldname, hash naming, and explicit id preservation
//! all require either DB counter tables or actual document inserts.

use loom_tests::*;
use serde_json::json;
use std::collections::HashSet;

use loom_core::doctype::controller;
use loom_core::doctype::meta::*;
use loom_core::doctype::naming::resolve_name;

#[tokio::test]
async fn test_series_naming_increments() {
    skip_without_db!();

    let meta = Meta {
        name: "Series Test".to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Series,
        autoname: Some("TEST-#####".to_string()),
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            label: Some("Title".into()),
            fieldtype: FieldType::Data,
            reqd: true,
            ..DocFieldMeta::default()
        }],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    };

    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    // Insert 3 documents and verify series naming increments
    let mut names = Vec::new();
    for i in 1..=3 {
        let mut doc = json!({ "title": format!("Series Doc {}", i) });
        let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
            .await
            .unwrap();
        let id = result
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();
        names.push(id);
    }

    assert_eq!(names[0], "TEST-00001");
    assert_eq!(names[1], "TEST-00002");
    assert_eq!(names[2], "TEST-00003");
}

#[tokio::test]
async fn test_prompt_naming_uses_provided_id() {
    skip_without_db!();

    let meta = Meta {
        name: "Prompt Test".to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Prompt,
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            label: Some("Title".into()),
            fieldtype: FieldType::Data,
            reqd: true,
            ..DocFieldMeta::default()
        }],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    };

    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "id": "my-custom-id", "title": "Prompt Doc" });
    let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();

    assert_eq!(
        result.get("id").and_then(|v| v.as_str()).unwrap(),
        "my-custom-id"
    );
}

#[tokio::test]
async fn test_prompt_naming_missing_id_fails() {
    skip_without_db!();

    let meta = Meta {
        name: "Prompt Fail".to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Prompt,
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            label: Some("Title".into()),
            fieldtype: FieldType::Data,
            reqd: true,
            ..DocFieldMeta::default()
        }],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    };

    let (db, _registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // resolve_name with no id should fail for Prompt naming
    let doc = json!({ "title": "No ID Provided" });
    let result = resolve_name(&meta, &doc, &db.pool).await;
    assert!(
        result.is_err(),
        "prompt naming without id should return an error"
    );
}

#[tokio::test]
async fn test_by_fieldname_naming() {
    skip_without_db!();

    let meta = Meta {
        name: "ByField Test".to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::ByFieldname,
        autoname: Some("title".to_string()),
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            label: Some("Title".into()),
            fieldtype: FieldType::Data,
            reqd: true,
            ..DocFieldMeta::default()
        }],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    };

    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "title": "My Doc Title" });
    let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();

    assert_eq!(
        result.get("id").and_then(|v| v.as_str()).unwrap(),
        "My Doc Title"
    );
}

#[tokio::test]
async fn test_hash_naming_unique_across_inserts() {
    skip_without_db!();

    let meta = Meta {
        name: "Hash Uniq".to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Hash,
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            label: Some("Title".into()),
            fieldtype: FieldType::Data,
            reqd: true,
            ..DocFieldMeta::default()
        }],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    };

    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut ids = HashSet::new();
    for i in 1..=10 {
        let mut doc = json!({ "title": format!("Hash Doc {}", i) });
        let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
            .await
            .unwrap();
        let id = result
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();
        assert!(ids.insert(id.clone()), "Duplicate ID generated: {}", id);
    }
    assert_eq!(ids.len(), 10);
}

#[tokio::test]
async fn test_explicit_id_preserved() {
    skip_without_db!();

    // Even with Hash naming, if doc already has an id, it should be preserved
    let meta = Meta {
        name: "Explicit ID".to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Hash,
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            label: Some("Title".into()),
            fieldtype: FieldType::Data,
            reqd: true,
            ..DocFieldMeta::default()
        }],
        permissions: vec![DocPermMeta {
            role: "Administrator".into(),
            read: true,
            write: true,
            create: true,
            delete: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    };

    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    let mut doc = json!({ "id": "explicit-name-001", "title": "Explicit" });
    let result = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();

    assert_eq!(
        result.get("id").and_then(|v| v.as_str()).unwrap(),
        "explicit-name-001",
        "explicit id should be preserved even with Hash naming rule"
    );
}
