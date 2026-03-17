use loom_core::doctype::workflow::*;

fn test_workflow() -> Workflow {
    Workflow {
        name: "Leave Approval".into(),
        document_type: "Leave Application".into(),
        is_active: true,
        states: vec![
            WorkflowState {
                state: "Draft".into(),
                doc_status: 0,
                allow_edit: None,
            },
            WorkflowState {
                state: "Pending".into(),
                doc_status: 0,
                allow_edit: Some("HR Manager".into()),
            },
            WorkflowState {
                state: "Approved".into(),
                doc_status: 1,
                allow_edit: None,
            },
            WorkflowState {
                state: "Rejected".into(),
                doc_status: 0,
                allow_edit: None,
            },
            WorkflowState {
                state: "Cancelled".into(),
                doc_status: 2,
                allow_edit: None,
            },
        ],
        transitions: vec![
            WorkflowTransition {
                state: "Draft".into(),
                action: "Submit".into(),
                next_state: "Pending".into(),
                allowed: "Employee".into(),
                condition: None,
            },
            WorkflowTransition {
                state: "Pending".into(),
                action: "Approve".into(),
                next_state: "Approved".into(),
                allowed: "HR Manager".into(),
                condition: None,
            },
            WorkflowTransition {
                state: "Pending".into(),
                action: "Reject".into(),
                next_state: "Rejected".into(),
                allowed: "HR Manager".into(),
                condition: None,
            },
            WorkflowTransition {
                state: "Approved".into(),
                action: "Cancel".into(),
                next_state: "Cancelled".into(),
                allowed: "HR Manager".into(),
                condition: None,
            },
        ],
    }
}

#[test]
fn test_valid_transition() {
    let wf = test_workflow();
    let roles = vec!["Employee".to_string()];
    let result = wf.validate_transition("Draft", "Submit", &roles);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().next_state, "Pending");
}

#[test]
fn test_invalid_transition_wrong_state() {
    let wf = test_workflow();
    let roles = vec!["Employee".to_string()];
    let result = wf.validate_transition("Approved", "Submit", &roles);
    assert!(result.is_err());
}

#[test]
fn test_invalid_transition_wrong_role() {
    let wf = test_workflow();
    let roles = vec!["Employee".to_string()]; // Not HR Manager
    let result = wf.validate_transition("Pending", "Approve", &roles);
    assert!(result.is_err());
}

#[test]
fn test_valid_transition_hr_manager() {
    let wf = test_workflow();
    let roles = vec!["HR Manager".to_string()];

    let approve = wf.validate_transition("Pending", "Approve", &roles);
    assert!(approve.is_ok());
    assert_eq!(approve.unwrap().next_state, "Approved");

    let reject = wf.validate_transition("Pending", "Reject", &roles);
    assert!(reject.is_ok());
    assert_eq!(reject.unwrap().next_state, "Rejected");
}

#[test]
fn test_cancel_transition() {
    let wf = test_workflow();
    let roles = vec!["HR Manager".to_string()];
    let result = wf.validate_transition("Approved", "Cancel", &roles);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().next_state, "Cancelled");
}

#[test]
fn test_initial_state() {
    let wf = test_workflow();
    let initial = wf.initial_state();
    assert!(initial.is_some());
    assert_eq!(initial.unwrap().state, "Draft");
}

#[test]
fn test_nonexistent_action() {
    let wf = test_workflow();
    let roles = vec!["Employee".to_string()];
    let result = wf.validate_transition("Draft", "Delete", &roles);
    assert!(result.is_err());
}
