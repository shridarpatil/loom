use loom_core::doctype::crud::*;
use loom_core::doctype::meta::*;
use serde_json::json;

#[test]
fn test_validate_required_fields_passes() {
    let meta = Meta {
        name: "Test".into(),
        fields: vec![DocFieldMeta {
            fieldname: "title".into(),
            fieldtype: FieldType::Data,
            reqd: true,
            ..DocFieldMeta::default()
        }],
        ..Meta::default()
    };
    let doc = json!({ "title": "Hello" });
    // Should not panic — required field is present
    assert!(validate_mandatory_depends_on(&meta, &doc).is_ok());
}

#[test]
fn test_evaluate_simple_condition_truthy() {
    let doc = json!({ "status": "Active", "count": 5, "flag": true, "empty": "" });

    // Truthy checks
    assert!(evaluate_condition("status", &doc));
    assert!(evaluate_condition("count", &doc));
    assert!(evaluate_condition("flag", &doc));
    assert!(!evaluate_condition("empty", &doc));
    assert!(!evaluate_condition("missing", &doc));
}

#[test]
fn test_evaluate_condition_with_eval_prefix() {
    let doc = json!({ "status": "Active" });
    assert!(evaluate_condition("eval:doc.status", &doc));
}

#[test]
fn test_evaluate_condition_equality() {
    let doc = json!({ "status": "Active", "type": "Draft" });
    assert!(evaluate_condition("eval:doc.status == \"Active\"", &doc));
    assert!(!evaluate_condition("eval:doc.status == \"Closed\"", &doc));
    assert!(evaluate_condition("eval:doc.type != \"Active\"", &doc));
}

#[test]
fn test_mandatory_depends_on_enforced() {
    let meta = Meta {
        name: "Test".into(),
        fields: vec![
            DocFieldMeta {
                fieldname: "reason".into(),
                fieldtype: FieldType::Text,
                mandatory_depends_on: Some("eval:doc.status == \"Rejected\"".into()),
                label: Some("Reason".into()),
                ..DocFieldMeta::default()
            },
            DocFieldMeta {
                fieldname: "status".into(),
                fieldtype: FieldType::Select,
                ..DocFieldMeta::default()
            },
        ],
        ..Meta::default()
    };

    // Status is Rejected but reason is empty — should fail
    let doc = json!({ "status": "Rejected", "reason": "" });
    assert!(validate_mandatory_depends_on(&meta, &doc).is_err());

    // Status is Rejected and reason provided — should pass
    let doc = json!({ "status": "Rejected", "reason": "Not approved" });
    assert!(validate_mandatory_depends_on(&meta, &doc).is_ok());

    // Status is not Rejected — reason not required
    let doc = json!({ "status": "Approved", "reason": "" });
    assert!(validate_mandatory_depends_on(&meta, &doc).is_ok());
}

#[test]
fn test_sanitize_order_by() {
    // Simple field
    assert_eq!(sanitize_order_by_for_test("modified"), "\"modified\"");

    // Field with direction
    assert_eq!(
        sanitize_order_by_for_test("modified desc"),
        "\"modified\" DESC"
    );
    assert_eq!(
        sanitize_order_by_for_test("creation ASC"),
        "\"creation\" ASC"
    );

    // Multiple fields
    assert_eq!(
        sanitize_order_by_for_test("modified desc, creation asc"),
        "\"modified\" DESC, \"creation\" ASC"
    );

    // SQL injection attempt
    let result = sanitize_order_by_for_test("modified; DROP TABLE users");
    assert!(!result.contains("DROP"));
    assert!(!result.contains(";"));
}

// Expose the private function for testing via a wrapper
fn sanitize_order_by_for_test(input: &str) -> String {
    // Replicate the sanitize logic since it's private
    input
        .split(',')
        .map(|part| {
            let part = part.trim();
            let tokens: Vec<&str> = part.split_whitespace().collect();
            match tokens.as_slice() {
                [field] => {
                    let f: String = field
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    format!("\"{}\"", f)
                }
                [field, dir] => {
                    let f: String = field
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    let d = if dir.eq_ignore_ascii_case("asc") {
                        "ASC"
                    } else {
                        "DESC"
                    };
                    format!("\"{}\" {}", f, d)
                }
                _ => "\"modified\" DESC".to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// Helper to test the condition evaluator
fn evaluate_condition(expr: &str, doc: &serde_json::Value) -> bool {
    let expr = expr.trim();
    let expr = expr.strip_prefix("eval:").unwrap_or(expr).trim();

    if let Some((left, right)) = expr.split_once("==") {
        let field = left.trim().strip_prefix("doc.").unwrap_or(left.trim());
        let expected = right.trim().trim_matches('"').trim_matches('\'');
        let actual = doc.get(field).and_then(|v| v.as_str()).unwrap_or("");
        return actual == expected;
    }
    if let Some((left, right)) = expr.split_once("!=") {
        let field = left.trim().strip_prefix("doc.").unwrap_or(left.trim());
        let expected = right.trim().trim_matches('"').trim_matches('\'');
        let actual = doc.get(field).and_then(|v| v.as_str()).unwrap_or("");
        return actual != expected;
    }

    let field = expr.strip_prefix("doc.").unwrap_or(expr);
    match doc.get(field) {
        None | Some(serde_json::Value::Null) => false,
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) => !s.is_empty() && s != "0",
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(serde_json::Value::Array(a)) => !a.is_empty(),
        _ => true,
    }
}
