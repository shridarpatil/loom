use loom_core::doctype::child::*;
use serde_json::json;

#[test]
fn test_prepare_child_rows() {
    let mut rows = vec![
        json!({ "item": "Widget", "qty": 2 }),
        json!({ "item": "Gadget", "qty": 1 }),
    ];

    prepare_child_rows("INV-001", "Invoice", "items", &mut rows).unwrap();

    assert_eq!(rows[0]["parent"], "INV-001");
    assert_eq!(rows[0]["parentfield"], "items");
    assert_eq!(rows[0]["parenttype"], "Invoice");
    assert_eq!(rows[0]["idx"], 1);

    assert_eq!(rows[1]["parent"], "INV-001");
    assert_eq!(rows[1]["idx"], 2);
}

#[test]
fn test_prepare_child_rows_empty() {
    let mut rows = vec![];
    let result = prepare_child_rows("INV-001", "Invoice", "items", &mut rows);
    assert!(result.is_ok());
}

#[test]
fn test_extract_child_rows() {
    let doc = json!({
        "id": "INV-001",
        "items": [
            { "item": "A", "qty": 1 },
            { "item": "B", "qty": 2 },
        ]
    });

    let rows = extract_child_rows(&doc, "items");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["item"], "A");
    assert_eq!(rows[1]["item"], "B");
}

#[test]
fn test_extract_child_rows_missing_field() {
    let doc = json!({ "id": "INV-001" });
    let rows = extract_child_rows(&doc, "items");
    assert!(rows.is_empty());
}

#[test]
fn test_extract_child_rows_null_field() {
    let doc = json!({ "id": "INV-001", "items": null });
    let rows = extract_child_rows(&doc, "items");
    assert!(rows.is_empty());
}

#[test]
fn test_prepare_child_rows_invalid_row() {
    let mut rows = vec![json!("not an object")];
    let result = prepare_child_rows("P", "T", "f", &mut rows);
    assert!(result.is_err());
}
