//! Integration tests for workflow transitions with real database.
//! Tests that submitting documents with workflows correctly transitions state
//! and that role restrictions are enforced.

use loom_tests::*;
use serde_json::json;

use loom_core::doctype::controller;
use loom_core::doctype::meta::*;
use loom_core::doctype::workflow::*;

/// Build a submittable DocType with a multi-step workflow.
/// Workflow states:
///   Draft (docstatus=0) → Pending Approval (docstatus=0) → Approved (docstatus=1) → Cancelled (docstatus=2)
/// Transitions:
///   Draft --[Submit by Administrator]--> Pending Approval
///   Pending Approval --[Submit by Approver]--> Approved
///   Approved --[Cancel by Administrator]--> Cancelled
fn workflow_doctype() -> Meta {
    Meta {
        name: "WF Doc".to_string(),
        module: "Test".to_string(),
        is_submittable: true,
        naming_rule: NamingRule::Hash,
        fields: vec![
            DocFieldMeta {
                fieldname: "title".into(),
                label: Some("Title".into()),
                fieldtype: FieldType::Data,
                reqd: true,
                ..DocFieldMeta::default()
            },
            // workflow_state column is needed for the controller to read/write state
            DocFieldMeta {
                fieldname: "workflow_state".into(),
                label: Some("Workflow State".into()),
                fieldtype: FieldType::Data,
                ..DocFieldMeta::default()
            },
        ],
        permissions: vec![
            DocPermMeta {
                role: "Administrator".into(),
                read: true,
                write: true,
                create: true,
                submit: true,
                cancel: true,
                delete: true,
                ..DocPermMeta::default()
            },
            DocPermMeta {
                role: "Approver".into(),
                read: true,
                write: true,
                submit: true,
                ..DocPermMeta::default()
            },
        ],
        workflow: Some(Workflow {
            name: "WF Doc Workflow".to_string(),
            document_type: "WF Doc".to_string(),
            is_active: true,
            states: vec![
                WorkflowState {
                    state: "Draft".into(),
                    doc_status: 0,
                    allow_edit: Some("Administrator".into()),
                },
                WorkflowState {
                    state: "Pending Approval".into(),
                    doc_status: 0,
                    allow_edit: Some("Approver".into()),
                },
                WorkflowState {
                    state: "Approved".into(),
                    doc_status: 1,
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
                    next_state: "Pending Approval".into(),
                    allowed: "Administrator".into(),
                    condition: None,
                },
                WorkflowTransition {
                    state: "Pending Approval".into(),
                    action: "Submit".into(),
                    next_state: "Approved".into(),
                    allowed: "Approver".into(),
                    condition: None,
                },
                WorkflowTransition {
                    state: "Approved".into(),
                    action: "Cancel".into(),
                    next_state: "Cancelled".into(),
                    allowed: "Administrator".into(),
                    condition: None,
                },
            ],
        }),
        ..Meta::default()
    }
}

#[tokio::test]
async fn test_submit_with_workflow_transitions_state() {
    skip_without_db!();

    let meta = workflow_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;
    let ctx = system_ctx(&db.pool, registry);

    // Insert a document (starts in Draft state)
    let mut doc = json!({ "title": "Workflow Test Doc" });
    let inserted = controller::insert(&ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Verify initial docstatus is 0
    assert_eq!(
        inserted
            .get("docstatus")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1),
        0
    );

    // Submit as Administrator: Draft -> Pending Approval
    // Administrator has the "Submit" action from "Draft" state
    let submitted = controller::submit(&ctx, &meta, id, &noop_hooks())
        .await
        .unwrap();

    // The workflow transition from "Draft" via "Submit" by Administrator
    // goes to "Pending Approval" which has doc_status=0, but the submit
    // controller sets docstatus=1 along with workflow_state.
    // Actually, looking at the controller code, submit always sets docstatus=1,
    // and sets the workflow_state to the transition's next_state.
    let workflow_state = submitted
        .get("workflow_state")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(
        workflow_state, "Pending Approval",
        "workflow_state should be 'Pending Approval' after first submit"
    );
}

#[tokio::test]
async fn test_workflow_wrong_role_rejected() {
    skip_without_db!();

    let meta = workflow_doctype();
    let (db, registry) = TestDb::with_doctypes(vec![meta.clone()]).await;

    // Insert as admin
    let admin_ctx = system_ctx(&db.pool, registry.clone());
    let mut doc = json!({ "title": "Wrong Role Test" });
    let inserted = controller::insert(&admin_ctx, &meta, &mut doc, &noop_hooks())
        .await
        .unwrap();
    let id = inserted.get("id").and_then(|v| v.as_str()).unwrap();

    // Submit as admin first (Draft -> Pending Approval)
    controller::submit(&admin_ctx, &meta, id, &noop_hooks())
        .await
        .unwrap();

    // Now try to submit again from "Pending Approval" as a user who is NOT an Approver.
    // The transition from "Pending Approval" via "Submit" requires the "Approver" role.
    // A user with only "HR User" role should be rejected.
    let non_approver_ctx = user_ctx(
        &db.pool,
        registry.clone(),
        "hruser@test.com",
        vec!["HR User".to_string(), "All".to_string()],
    );

    // Attempting submit from "Pending Approval" without "Approver" role should fail.
    // Note: The submit controller also checks Submit permission, so "HR User" might
    // fail at the permission check first. Either way, the operation should be rejected.
    let result = controller::submit(&non_approver_ctx, &meta, id, &noop_hooks()).await;
    assert!(
        result.is_err(),
        "submit from Pending Approval without Approver role should fail"
    );
}
