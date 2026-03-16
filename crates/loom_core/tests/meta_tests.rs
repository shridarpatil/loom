use loom_core::doctype::meta::*;

#[test]
fn test_table_name_simple() {
    assert_eq!(doctype_table_name("User"), "user");
    assert_eq!(doctype_table_name("Role"), "role");
    assert_eq!(doctype_table_name("Todo"), "todo");
}

#[test]
fn test_table_name_multi_word() {
    assert_eq!(doctype_table_name("Leave Application"), "leave_application");
    assert_eq!(doctype_table_name("Invoice Item"), "invoice_item");
    assert_eq!(doctype_table_name("Todo Category"), "todo_category");
}

#[test]
fn test_table_name_camel_case() {
    assert_eq!(doctype_table_name("DocType"), "doc_type");
    assert_eq!(doctype_table_name("TodoItem"), "todo_item");
}

#[test]
fn test_table_name_abbreviations() {
    assert_eq!(doctype_table_name("HR"), "hr");
}

#[test]
fn test_merge_permissions_empty_override() {
    let defaults = vec![DocPermMeta {
        role: "All".into(),
        permlevel: 0,
        read: true,
        write: true,
        ..DocPermMeta::default()
    }];
    let result = merge_permission_overrides(&defaults, &[]);
    assert_eq!(result.len(), 1);
    assert!(result[0].read);
    assert!(result[0].write);
}

#[test]
fn test_merge_permissions_override_replaces() {
    let defaults = vec![DocPermMeta {
        role: "All".into(),
        permlevel: 0,
        read: true,
        write: true,
        ..DocPermMeta::default()
    }];
    let overrides = vec![DocPermMeta {
        role: "All".into(),
        permlevel: 0,
        read: true,
        write: false, // changed
        ..DocPermMeta::default()
    }];
    let result = merge_permission_overrides(&defaults, &overrides);
    assert_eq!(result.len(), 1);
    assert!(result[0].read);
    assert!(!result[0].write); // overridden
}

#[test]
fn test_merge_permissions_adds_new_rule() {
    let defaults = vec![DocPermMeta {
        role: "All".into(),
        permlevel: 0,
        read: true,
        ..DocPermMeta::default()
    }];
    let overrides = vec![DocPermMeta {
        role: "HR Manager".into(),
        permlevel: 1,
        read: true,
        write: true,
        ..DocPermMeta::default()
    }];
    let result = merge_permission_overrides(&defaults, &overrides);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].role, "All");
    assert_eq!(result[1].role, "HR Manager");
}

#[test]
fn test_merge_permissions_preserves_untouched_defaults() {
    let defaults = vec![
        DocPermMeta { role: "All".into(), permlevel: 0, read: true, write: true, ..DocPermMeta::default() },
        DocPermMeta { role: "Admin".into(), permlevel: 0, read: true, write: true, create: true, ..DocPermMeta::default() },
    ];
    let overrides = vec![DocPermMeta {
        role: "All".into(),
        permlevel: 0,
        read: true,
        write: false,
        ..DocPermMeta::default()
    }];
    let result = merge_permission_overrides(&defaults, &overrides);
    assert_eq!(result.len(), 2);
    assert!(!result[0].write); // overridden
    assert!(result[1].create); // preserved
}

#[test]
fn test_meta_from_json() {
    let json = r#"{
        "name": "Test",
        "module": "Core",
        "fields": [
            {"fieldname": "title", "fieldtype": "Data", "reqd": true}
        ],
        "permissions": [
            {"role": "All", "read": true, "write": true}
        ]
    }"#;
    let meta = Meta::from_json(json).unwrap();
    assert_eq!(meta.name, "Test");
    assert_eq!(meta.fields.len(), 1);
    assert_eq!(meta.fields[0].fieldname, "title");
    assert!(meta.fields[0].reqd);
    assert_eq!(meta.permissions.len(), 1);
}

#[test]
fn test_meta_data_fields_excludes_layout() {
    let json = r#"{
        "name": "Test",
        "module": "Core",
        "fields": [
            {"fieldname": "title", "fieldtype": "Data"},
            {"fieldname": "sb", "fieldtype": "SectionBreak"},
            {"fieldname": "cb", "fieldtype": "ColumnBreak"},
            {"fieldname": "desc", "fieldtype": "Text"}
        ]
    }"#;
    let meta = Meta::from_json(json).unwrap();
    let data_fields: Vec<_> = meta.data_fields().collect();
    assert_eq!(data_fields.len(), 2);
    assert_eq!(data_fields[0].fieldname, "title");
    assert_eq!(data_fields[1].fieldname, "desc");
}

#[test]
fn test_set_standard_fields_on_insert() {
    let mut doc = serde_json::json!({ "title": "Test" });
    set_standard_fields_on_insert(&mut doc, "admin@test.com");
    assert_eq!(doc["owner"], "admin@test.com");
    assert_eq!(doc["modified_by"], "admin@test.com");
    assert_eq!(doc["docstatus"], 0);
    assert!(doc["creation"].as_str().is_some());
    assert!(doc["modified"].as_str().is_some());
}

#[test]
fn test_set_standard_fields_on_update() {
    let mut doc = serde_json::json!({ "title": "Test", "owner": "original@test.com" });
    set_standard_fields_on_update(&mut doc, "editor@test.com");
    assert_eq!(doc["modified_by"], "editor@test.com");
    assert_eq!(doc["owner"], "original@test.com"); // owner not changed
    assert!(doc["modified"].as_str().is_some());
}
