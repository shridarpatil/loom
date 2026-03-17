use loom_core::db::query::{FilterOp, QueryBuilder};
use serde_json::json;

// ===========================================================================
// FilterOp::as_sql tests
// ===========================================================================

#[test]
fn test_filter_op_as_sql() {
    assert_eq!(FilterOp::Eq.as_sql(), "=");
    assert_eq!(FilterOp::Ne.as_sql(), "!=");
    assert_eq!(FilterOp::Gt.as_sql(), ">");
    assert_eq!(FilterOp::Lt.as_sql(), "<");
    assert_eq!(FilterOp::Gte.as_sql(), ">=");
    assert_eq!(FilterOp::Lte.as_sql(), "<=");
    assert_eq!(FilterOp::Like.as_sql(), "LIKE");
    assert_eq!(FilterOp::NotLike.as_sql(), "NOT LIKE");
    assert_eq!(FilterOp::In.as_sql(), "IN");
    assert_eq!(FilterOp::NotIn.as_sql(), "NOT IN");
    assert_eq!(FilterOp::IsNull.as_sql(), "IS NULL");
    assert_eq!(FilterOp::IsNotNull.as_sql(), "IS NOT NULL");
    assert_eq!(FilterOp::Between.as_sql(), "BETWEEN");
}

// ===========================================================================
// FilterOp::from_str tests
// ===========================================================================

#[test]
fn test_filter_op_from_str_all_ops() {
    // Eq
    assert!(matches!(FilterOp::from_str("="), Some(FilterOp::Eq)));

    // Ne — both aliases
    assert!(matches!(FilterOp::from_str("!="), Some(FilterOp::Ne)));
    assert!(matches!(FilterOp::from_str("<>"), Some(FilterOp::Ne)));

    // Gt, Lt, Gte, Lte
    assert!(matches!(FilterOp::from_str(">"), Some(FilterOp::Gt)));
    assert!(matches!(FilterOp::from_str("<"), Some(FilterOp::Lt)));
    assert!(matches!(FilterOp::from_str(">="), Some(FilterOp::Gte)));
    assert!(matches!(FilterOp::from_str("<="), Some(FilterOp::Lte)));

    // Like — case variants
    assert!(matches!(FilterOp::from_str("like"), Some(FilterOp::Like)));
    assert!(matches!(FilterOp::from_str("LIKE"), Some(FilterOp::Like)));

    // NotLike — case variants
    assert!(matches!(
        FilterOp::from_str("not like"),
        Some(FilterOp::NotLike)
    ));
    assert!(matches!(
        FilterOp::from_str("NOT LIKE"),
        Some(FilterOp::NotLike)
    ));

    // In — case variants
    assert!(matches!(FilterOp::from_str("in"), Some(FilterOp::In)));
    assert!(matches!(FilterOp::from_str("IN"), Some(FilterOp::In)));

    // NotIn — case variants
    assert!(matches!(
        FilterOp::from_str("not in"),
        Some(FilterOp::NotIn)
    ));
    assert!(matches!(
        FilterOp::from_str("NOT IN"),
        Some(FilterOp::NotIn)
    ));

    // IsNull — case variants
    assert!(matches!(FilterOp::from_str("is"), Some(FilterOp::IsNull)));
    assert!(matches!(FilterOp::from_str("IS"), Some(FilterOp::IsNull)));

    // IsNotNull — case variants
    assert!(matches!(
        FilterOp::from_str("is not"),
        Some(FilterOp::IsNotNull)
    ));
    assert!(matches!(
        FilterOp::from_str("IS NOT"),
        Some(FilterOp::IsNotNull)
    ));

    // Between — case variants
    assert!(matches!(
        FilterOp::from_str("between"),
        Some(FilterOp::Between)
    ));
    assert!(matches!(
        FilterOp::from_str("BETWEEN"),
        Some(FilterOp::Between)
    ));
}

#[test]
fn test_filter_op_from_str_unknown() {
    assert!(FilterOp::from_str("CONTAINS").is_none());
    assert!(FilterOp::from_str("").is_none());
    assert!(FilterOp::from_str("equals").is_none());
    assert!(FilterOp::from_str("~~").is_none());
}

// ===========================================================================
// QueryBuilder tests
// ===========================================================================

#[test]
fn test_query_builder_defaults() {
    let qb = QueryBuilder::new("My DocType");
    assert_eq!(qb.table, "my_doc_type");
    assert_eq!(qb.select_fields, vec!["*".to_string()]);

    let (sql, binds) = qb.build();
    // Default limit is 20
    assert!(
        sql.contains("LIMIT 20"),
        "Expected default LIMIT 20 in: {}",
        sql
    );
    assert!(binds.is_empty());
}

#[test]
fn test_query_builder_select_fields() {
    let qb = QueryBuilder::new("Employee").fields(&["name", "title"]);
    let (sql, _) = qb.build();

    assert!(
        sql.contains("\"name\""),
        "Expected quoted field 'name' in: {}",
        sql
    );
    assert!(
        sql.contains("\"title\""),
        "Expected quoted field 'title' in: {}",
        sql
    );
    // Should NOT contain the wildcard since we specified fields
    assert!(
        !sql.starts_with("SELECT *"),
        "Should not select * when fields are specified: {}",
        sql
    );
}

#[test]
fn test_query_builder_single_filter() {
    let qb = QueryBuilder::new("Task").filter("status", FilterOp::Eq, json!("Open"));
    let (sql, binds) = qb.build();

    assert!(
        sql.contains("WHERE \"status\" = $1"),
        "Expected WHERE clause in: {}",
        sql
    );
    assert_eq!(binds.len(), 1);
    assert_eq!(binds[0], json!("Open"));
}

#[test]
fn test_query_builder_multiple_filters() {
    let qb = QueryBuilder::new("Task")
        .filter("status", FilterOp::Eq, json!("Open"))
        .filter("priority", FilterOp::Gt, json!(3));
    let (sql, binds) = qb.build();

    assert!(
        sql.contains("\"status\" = $1"),
        "Expected first filter in: {}",
        sql
    );
    assert!(
        sql.contains("\"priority\" > $2"),
        "Expected second filter in: {}",
        sql
    );
    assert!(
        sql.contains(" AND "),
        "Expected AND between filters in: {}",
        sql
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0], json!("Open"));
    assert_eq!(binds[1], json!(3));
}

#[test]
fn test_query_builder_in_filter() {
    let qb = QueryBuilder::new("Task").filter("status", FilterOp::In, json!(["Open", "Closed"]));
    let (sql, binds) = qb.build();

    assert!(
        sql.contains("\"status\" IN ($1, $2)"),
        "Expected IN clause with two placeholders in: {}",
        sql
    );
    assert_eq!(binds.len(), 2);
    assert_eq!(binds[0], json!("Open"));
    assert_eq!(binds[1], json!("Closed"));
}

#[test]
fn test_query_builder_is_null_filter() {
    let qb = QueryBuilder::new("Task").filter("deleted", FilterOp::IsNull, json!(null));
    let (sql, binds) = qb.build();

    assert!(
        sql.contains("\"deleted\" IS NULL"),
        "Expected IS NULL in: {}",
        sql
    );
    // IS NULL should not add any bind values
    assert!(
        binds.is_empty(),
        "IS NULL should produce no bind values, got: {:?}",
        binds
    );
}

#[test]
fn test_query_builder_order_by() {
    let qb = QueryBuilder::new("Task").order_by("\"creation\" ASC");
    let (sql, _) = qb.build();

    assert!(
        sql.contains("ORDER BY \"creation\" ASC"),
        "Expected custom ORDER BY in: {}",
        sql
    );
}

#[test]
fn test_query_builder_limit_offset() {
    let qb = QueryBuilder::new("Task").limit(10).offset(20);
    let (sql, _) = qb.build();

    assert!(sql.contains("LIMIT 10"), "Expected LIMIT 10 in: {}", sql);
    assert!(sql.contains("OFFSET 20"), "Expected OFFSET 20 in: {}", sql);
}

#[test]
fn test_query_builder_default_limit() {
    // When no limit is explicitly set, the default should be 20
    let qb = QueryBuilder::new("Task");
    let (sql, _) = qb.build();

    assert!(
        sql.contains("LIMIT 20"),
        "Expected default LIMIT 20 in: {}",
        sql
    );
}

#[test]
fn test_query_builder_default_order() {
    // When no order_by is set, the default should be "modified" DESC
    let qb = QueryBuilder::new("Task");
    let (sql, _) = qb.build();

    assert!(
        sql.contains("ORDER BY \"modified\" DESC"),
        "Expected default ORDER BY \"modified\" DESC in: {}",
        sql
    );
}
