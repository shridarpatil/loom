use loom_core::doctype::meta::*;
use loom_core::doctype::naming::generate_name;
use serde_json::json;

/// Helper: build a Meta with Expression naming rule and the given autoname pattern.
fn expression_meta(autoname: Option<&str>) -> Meta {
    Meta {
        name: "Test DocType".to_string(),
        module: "Test".to_string(),
        naming_rule: NamingRule::Expression,
        autoname: autoname.map(|s| s.to_string()),
        ..Meta::default()
    }
}

#[test]
fn test_expression_with_year() {
    let meta = expression_meta(Some("INV-.YYYY.-"));
    let doc = json!({});

    let name = generate_name(&meta, &doc).expect("generate_name should succeed");

    let current_year = chrono::Utc::now().format("%Y").to_string();
    let expected = format!("INV-{}-", current_year);
    assert_eq!(
        name, expected,
        "Expression with .YYYY. should resolve to current year"
    );
}

#[test]
fn test_expression_with_field_ref() {
    let meta = expression_meta(Some("{department}-{code}"));
    let doc = json!({
        "department": "Engineering",
        "code": "ENG001"
    });

    let name = generate_name(&meta, &doc).expect("generate_name should succeed");
    assert_eq!(
        name, "Engineering-ENG001",
        "Expression should resolve field references from the document"
    );
}

#[test]
fn test_expression_missing_autoname() {
    let meta = expression_meta(None);
    let doc = json!({});

    let result = generate_name(&meta, &doc);
    assert!(
        result.is_err(),
        "Expression naming with no autoname should return an error"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("expression") || err_msg.contains("autoname"),
        "Error should mention expression or autoname: {}",
        err_msg
    );
}

#[test]
fn test_expression_with_date_placeholders() {
    let meta = expression_meta(Some("DOC-.YY.-.MM.-.DD.-"));
    let doc = json!({});

    let name = generate_name(&meta, &doc).expect("generate_name should succeed");

    let now = chrono::Utc::now();
    let expected_yy = now.format("%y").to_string();
    let expected_mm = now.format("%m").to_string();
    let expected_dd = now.format("%d").to_string();

    let expected = format!("DOC-{}-{}-{}-", expected_yy, expected_mm, expected_dd);
    assert_eq!(
        name, expected,
        "Expression with .YY., .MM., .DD. should all be resolved"
    );
}

#[test]
fn test_expression_with_series_hashes() {
    // The ##### pattern is left for the DB layer to resolve;
    // generate_name should return the string with # characters intact.
    let meta = expression_meta(Some("HR-.YYYY.-.#####"));
    let doc = json!({});

    let name = generate_name(&meta, &doc).expect("generate_name should succeed");

    let current_year = chrono::Utc::now().format("%Y").to_string();
    assert!(
        name.contains(&current_year),
        "Should contain the current year: {}",
        name
    );
    assert!(
        name.contains("#####"),
        "##### pattern should be preserved for DB-layer resolution: {}",
        name
    );
}

#[test]
fn test_expression_mixed_fields_and_dates() {
    let meta = expression_meta(Some("{company}-.YYYY.-{category}"));
    let doc = json!({
        "company": "Acme",
        "category": "Sales"
    });

    let name = generate_name(&meta, &doc).expect("generate_name should succeed");

    let current_year = chrono::Utc::now().format("%Y").to_string();
    let expected = format!("Acme-{}-Sales", current_year);
    assert_eq!(
        name, expected,
        "Expression should resolve both field refs and date placeholders"
    );
}
