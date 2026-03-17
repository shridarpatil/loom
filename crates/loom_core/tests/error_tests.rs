use loom_core::LoomError;

#[test]
fn test_error_types() {
    let err = LoomError::Validation("Field required".into());
    assert_eq!(err.error_type(), "ValidationError");
    assert!(err.to_string().contains("Field required"));

    let err = LoomError::NotFound {
        doctype: "Todo".into(),
        name: "1".into(),
    };
    assert_eq!(err.error_type(), "NotFoundError");
    assert!(err.to_string().contains("Todo"));
    assert!(err.to_string().contains("1"));

    let err = LoomError::PermissionDenied("No read access".into());
    assert_eq!(err.error_type(), "PermissionError");

    let err = LoomError::DuplicateEntry("name already exists".into());
    assert_eq!(err.error_type(), "DuplicateEntryError");

    let err = LoomError::LinkValidation {
        doctype: "Employee".into(),
        fieldname: "department".into(),
        value: "INVALID".into(),
    };
    assert_eq!(err.error_type(), "LinkValidationError");
    assert!(err.to_string().contains("department"));

    let err = LoomError::Script("syntax error".into());
    assert_eq!(err.error_type(), "ScriptError");

    let err = LoomError::Internal("unexpected".into());
    assert_eq!(err.error_type(), "InternalError");
}

#[test]
fn test_error_response() {
    let err = LoomError::Validation("bad input".into());
    let resp = err.to_response();
    assert_eq!(resp.error_type, "ValidationError");
    assert_eq!(resp.error, "Validation: bad input");
}
