use loom_core::doctype::meta::*;
use loom_core::doctype::naming::generate_name;
use serde_json::json;

fn make_meta(rule: NamingRule, autoname: Option<&str>) -> Meta {
    Meta {
        name: "Test".into(),
        naming_rule: rule,
        autoname: autoname.map(|s| s.to_string()),
        fields: vec![
            DocFieldMeta {
                fieldname: "title".into(),
                fieldtype: FieldType::Data,
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "code".into(),
                fieldtype: FieldType::Data,
                ..DocFieldMeta::default()
            },
        ],
        ..Meta::default()
    }
}

#[test]
fn test_hash_naming_length() {
    let meta = make_meta(NamingRule::Hash, None);
    let name = generate_name(&meta, &json!({})).unwrap();
    assert_eq!(name.len(), 10);
    assert!(name.chars().all(|c| c.is_alphanumeric()));
}

#[test]
fn test_hash_naming_unique() {
    let meta = make_meta(NamingRule::Hash, None);
    let name1 = generate_name(&meta, &json!({})).unwrap();
    let name2 = generate_name(&meta, &json!({})).unwrap();
    assert_ne!(name1, name2);
}

#[test]
fn test_prompt_naming_uses_id() {
    let meta = make_meta(NamingRule::Prompt, None);
    let doc = json!({ "id": "MY-DOC-001" });
    let name = generate_name(&meta, &doc).unwrap();
    assert_eq!(name, "MY-DOC-001");
}

#[test]
fn test_prompt_naming_requires_id_field() {
    let meta = make_meta(NamingRule::Prompt, None);
    // "name" field doesn't count — only "id" works for prompt naming
    let doc = json!({ "name": "MY-NAME" });
    let result = generate_name(&meta, &doc);
    assert!(result.is_err());
}

#[test]
fn test_prompt_naming_empty_fails() {
    let meta = make_meta(NamingRule::Prompt, None);
    let doc = json!({});
    let result = generate_name(&meta, &doc);
    assert!(result.is_err());
}

#[test]
fn test_by_fieldname() {
    let mut meta = make_meta(NamingRule::ByFieldname, None);
    meta.autoname = Some("code".into());
    let doc = json!({ "code": "ABC-123" });
    let name = generate_name(&meta, &doc).unwrap();
    assert_eq!(name, "ABC-123");
}

#[test]
fn test_by_fieldname_empty_generates_hash() {
    let mut meta = make_meta(NamingRule::ByFieldname, None);
    meta.autoname = Some("code".into());
    let doc = json!({ "code": "" });
    // Empty field value falls back to generating a name (not error)
    let result = generate_name(&meta, &doc);
    assert!(result.is_ok());
}
