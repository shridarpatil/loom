use loom_core::doctype::meta::*;
use loom_core::perms::user_perm::{build_user_perm_filters, UserPermission};

// ===========================================================================
// Helper: create a Meta with a single Link field
// ===========================================================================

fn meta_with_link(name: &str, link_field: &str, link_to: &str) -> Meta {
    Meta {
        name: name.to_string(),
        module: "Test".to_string(),
        fields: vec![DocFieldMeta {
            fieldname: link_field.to_string(),
            label: Some(link_field.to_string()),
            fieldtype: FieldType::Link,
            options: Some(link_to.to_string()),
            ..DocFieldMeta::default()
        }],
        ..Meta::default()
    }
}

/// Helper: create a UserPermission
fn user_perm(allow: &str, for_value: &str, applicable_for: Option<&str>) -> UserPermission {
    UserPermission {
        user: "test@example.com".to_string(),
        allow: allow.to_string(),
        for_value: for_value.to_string(),
        applicable_for: applicable_for.map(|s| s.to_string()),
        is_default: false,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[test]
fn test_build_filters_empty_perms() {
    let meta = meta_with_link("Sales Order", "company", "Company");
    let perms: Vec<UserPermission> = vec![];

    let (clauses, binds) = build_user_perm_filters(&meta, &perms, "Sales Order");

    assert!(clauses.is_empty(), "No perms should produce no clauses");
    assert!(binds.is_empty(), "No perms should produce no bind values");
}

#[test]
fn test_build_filters_direct_doctype() {
    // When allow == the doctype being queried, filter on "id"
    let meta = Meta {
        name: "Company".to_string(),
        module: "Test".to_string(),
        fields: vec![],
        ..Meta::default()
    };
    let perms = vec![user_perm("Company", "Acme Corp", None)];

    let (clauses, binds) = build_user_perm_filters(&meta, &perms, "Company");

    assert_eq!(clauses.len(), 1);
    assert!(
        clauses[0].contains("\"id\" IN"),
        "Expected id IN clause, got: {}",
        clauses[0]
    );
    assert_eq!(binds, vec!["Acme Corp".to_string()]);
}

#[test]
fn test_build_filters_via_link_field() {
    // When allow is a different doctype, filter via the Link field
    let meta = meta_with_link("Sales Order", "company", "Company");
    let perms = vec![user_perm("Company", "Acme Corp", None)];

    let (clauses, binds) = build_user_perm_filters(&meta, &perms, "Sales Order");

    assert_eq!(clauses.len(), 1);
    assert!(
        clauses[0].contains("\"company\" IN"),
        "Expected company IN clause, got: {}",
        clauses[0]
    );
    assert_eq!(binds, vec!["Acme Corp".to_string()]);
}

#[test]
fn test_build_filters_applicable_for() {
    // A perm with applicable_for set should only match that specific doctype
    let meta = meta_with_link("Sales Order", "company", "Company");

    // This perm is only applicable for "Sales Order"
    let matching_perm = user_perm("Company", "Acme Corp", Some("Sales Order"));
    let (clauses, _) = build_user_perm_filters(&meta, &[matching_perm], "Sales Order");
    assert_eq!(
        clauses.len(),
        1,
        "Perm applicable_for='Sales Order' should apply to Sales Order"
    );

    // This perm is only applicable for "Purchase Order" — should NOT apply
    let non_matching_perm = user_perm("Company", "Acme Corp", Some("Purchase Order"));
    let (clauses, binds) = build_user_perm_filters(&meta, &[non_matching_perm], "Sales Order");
    assert!(
        clauses.is_empty(),
        "Perm applicable_for='Purchase Order' should NOT apply to Sales Order"
    );
    assert!(binds.is_empty());
}

#[test]
fn test_build_filters_multiple_values() {
    // Multiple perms for the same allow doctype should combine into a single IN list
    let meta = meta_with_link("Sales Order", "company", "Company");
    let perms = vec![
        user_perm("Company", "Acme Corp", None),
        user_perm("Company", "Beta Inc", None),
    ];

    let (clauses, binds) = build_user_perm_filters(&meta, &perms, "Sales Order");

    assert_eq!(clauses.len(), 1);
    assert!(
        clauses[0].contains("\"company\" IN"),
        "Expected company IN clause, got: {}",
        clauses[0]
    );
    // Both values should be in the bind list
    assert_eq!(binds.len(), 2);
    assert!(binds.contains(&"Acme Corp".to_string()));
    assert!(binds.contains(&"Beta Inc".to_string()));
}

#[test]
fn test_build_filters_no_matching_link() {
    // allow=DocType but no Link field in meta points to it → no filter added
    let meta = meta_with_link("Sales Order", "company", "Company");
    // Restrict "Territory" but Sales Order has no Link to Territory
    let perms = vec![user_perm("Territory", "North", None)];

    let (clauses, binds) = build_user_perm_filters(&meta, &perms, "Sales Order");

    assert!(
        clauses.is_empty(),
        "No Link field to Territory, so no filter should be produced"
    );
    assert!(binds.is_empty());
}
