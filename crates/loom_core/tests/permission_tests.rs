use loom_core::doctype::meta::*;
use loom_core::perms::check::*;
use loom_core::perms::PermType;

fn test_meta() -> Meta {
    Meta {
        name: "Test".into(),
        module: "Core".into(),
        permissions: vec![
            DocPermMeta {
                role: "All".into(),
                permlevel: 0,
                read: true,
                write: false,
                ..DocPermMeta::default()
            },
            DocPermMeta {
                role: "Editor".into(),
                permlevel: 0,
                read: true,
                write: true,
                create: true,
                ..DocPermMeta::default()
            },
            DocPermMeta {
                role: "Editor".into(),
                permlevel: 1,
                read: true,
                write: true,
                ..DocPermMeta::default()
            },
        ],
        fields: vec![
            DocFieldMeta { fieldname: "title".into(), fieldtype: FieldType::Data, permlevel: 0, ..DocFieldMeta::default() },
            DocFieldMeta { fieldname: "salary".into(), fieldtype: FieldType::Currency, permlevel: 1, ..DocFieldMeta::default() },
        ],
        ..Meta::default()
    }
}

#[test]
fn test_has_permission_basic() {
    let meta = test_meta();
    let roles = vec!["All".into()];
    assert!(has_permission(&meta, None, PermType::Read, "user@test.com", &roles));
    assert!(!has_permission(&meta, None, PermType::Write, "user@test.com", &roles));
}

#[test]
fn test_has_permission_editor() {
    let meta = test_meta();
    let roles = vec!["All".into(), "Editor".into()];
    assert!(has_permission(&meta, None, PermType::Read, "editor@test.com", &roles));
    assert!(has_permission(&meta, None, PermType::Write, "editor@test.com", &roles));
    assert!(has_permission(&meta, None, PermType::Create, "editor@test.com", &roles));
}

#[test]
fn test_has_permission_admin_bypass() {
    let meta = test_meta();
    let roles = vec!["Administrator".into()];
    assert!(has_permission(&meta, None, PermType::Read, "Administrator", &roles));
    assert!(has_permission(&meta, None, PermType::Write, "Administrator", &roles));
    assert!(has_permission(&meta, None, PermType::Delete, "Administrator", &roles));
}

#[test]
fn test_allowed_permlevels_read_includes_zero() {
    let meta = test_meta();
    // Editor has read at level 0 and level 1
    let roles = vec!["Editor".into()];
    let levels = allowed_permlevels(&meta, PermType::Read, &roles);
    assert!(levels.contains(&0));
    assert!(levels.contains(&1));
}

#[test]
fn test_allowed_permlevels_level1_only_includes_zero_for_read() {
    let meta = Meta {
        name: "Test".into(),
        permissions: vec![
            DocPermMeta {
                role: "Special".into(),
                permlevel: 1,
                read: true,
                ..DocPermMeta::default()
            },
        ],
        ..Meta::default()
    };
    let roles = vec!["Special".into()];
    let read_levels = allowed_permlevels(&meta, PermType::Read, &roles);
    assert!(read_levels.contains(&0)); // level 0 auto-included for read
    assert!(read_levels.contains(&1));

    let write_levels = allowed_permlevels(&meta, PermType::Write, &roles);
    assert!(write_levels.is_empty()); // no write at any level
}

#[test]
fn test_strip_fields_by_permlevel() {
    let meta = test_meta();
    let mut doc = serde_json::json!({
        "id": "1",
        "title": "Test",
        "salary": 50000,
        "owner": "admin",
    });

    let mut levels = std::collections::HashSet::new();
    levels.insert(0u8);
    // Only level 0 — salary (level 1) should be stripped
    strip_fields_by_permlevel(&mut doc, &meta, &levels);

    assert!(doc.get("title").is_some());
    assert!(doc.get("salary").is_none()); // stripped
    assert!(doc.get("id").is_some()); // standard field preserved
    assert!(doc.get("owner").is_some()); // standard field preserved
}

#[test]
fn test_check_write_permlevels() {
    let meta = test_meta();
    let mut levels = std::collections::HashSet::new();
    levels.insert(0u8);

    // Writing to level 0 field — OK
    let doc = serde_json::json!({ "title": "new title" });
    assert!(check_write_permlevels(&doc, &meta, &levels).is_ok());

    // Writing to level 1 field without permission — error
    let doc = serde_json::json!({ "salary": 60000 });
    assert!(check_write_permlevels(&doc, &meta, &levels).is_err());
}

#[test]
fn test_if_owner_permission() {
    let meta = Meta {
        name: "Test".into(),
        permissions: vec![DocPermMeta {
            role: "All".into(),
            permlevel: 0,
            read: true,
            if_owner: true,
            ..DocPermMeta::default()
        }],
        ..Meta::default()
    };
    let roles = vec!["All".into()];

    // Owner matches
    let doc = serde_json::json!({ "owner": "user@test.com" });
    assert!(has_permission(&meta, Some(&doc), PermType::Read, "user@test.com", &roles));

    // Owner doesn't match
    let doc = serde_json::json!({ "owner": "other@test.com" });
    assert!(!has_permission(&meta, Some(&doc), PermType::Read, "user@test.com", &roles));
}
