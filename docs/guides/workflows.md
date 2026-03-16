# Workflows

Workflows define state machines for submittable documents. They control who can transition a document between states.

## Defining a Workflow

Add a `workflow` field to your DocType JSON:

```json
{
  "name": "Leave Application",
  "module": "HR",
  "is_submittable": true,
  "workflow": {
    "name": "Leave Approval",
    "document_type": "Leave Application",
    "is_active": true,
    "states": [
      { "state": "Draft", "doc_status": 0 },
      { "state": "Pending Approval", "doc_status": 0, "allow_edit": "HR Manager" },
      { "state": "Approved", "doc_status": 1 },
      { "state": "Rejected", "doc_status": 0 },
      { "state": "Cancelled", "doc_status": 2 }
    ],
    "transitions": [
      { "state": "Draft", "action": "Submit", "next_state": "Pending Approval", "allowed": "Employee" },
      { "state": "Pending Approval", "action": "Approve", "next_state": "Approved", "allowed": "HR Manager" },
      { "state": "Pending Approval", "action": "Reject", "next_state": "Rejected", "allowed": "HR Manager" },
      { "state": "Approved", "action": "Cancel", "next_state": "Cancelled", "allowed": "HR Manager" }
    ]
  },
  "fields": [
    { "fieldname": "workflow_state", "label": "Workflow State", "fieldtype": "Data" },
    ...
  ]
}
```

## How It Works

- When a document is **submitted**, the workflow validates that a "Submit" transition exists from the current `workflow_state` and that the user has the required role
- When **cancelled**, it validates the "Cancel" transition
- The `workflow_state` field is updated to the `next_state` defined in the transition
- If no workflow is defined, submit/cancel work normally (just docstatus)

## Key Points

- Add a `workflow_state` field to your DocType to store the current state
- The first state in the `states` array is the initial state
- `doc_status` maps states to docstatus: 0 (Draft), 1 (Submitted), 2 (Cancelled)
- `allowed` specifies which role can perform the transition
- `allow_edit` on a state specifies which role can edit the document in that state
