# fetch_from & depends_on

## fetch_from — Auto-Populate from Linked Documents

When a document has a Link field, you can auto-fetch values from the linked document.

### Example

An Employee DocType has a `department` Link field. You want `department_name` to auto-fill:

```json
{
  "fieldname": "department",
  "label": "Department",
  "fieldtype": "Link",
  "options": "Department"
},
{
  "fieldname": "department_name",
  "label": "Department Name",
  "fieldtype": "Data",
  "read_only": true,
  "fetch_from": "department.department_name"
}
```

Format: `link_fieldname.source_fieldname`

When the user saves with `department = "DEP-001"`, the server fetches `department_name` from the Department document and sets it on the Employee.

### How It Works

- Runs before hooks (`validate`, `before_save`) on both insert and update
- Groups fetches by link field — multiple `fetch_from` fields pointing to the same link = one DB query
- Clears fetched fields when the link is emptied

### Frontend

The Form page also fetches on the client side when the Link field changes (for instant UI feedback). The server-side fetch is the authoritative one.

## mandatory_depends_on — Conditional Required Fields

Make a field required only when a condition is met.

```json
{
  "fieldname": "rejection_reason",
  "label": "Rejection Reason",
  "fieldtype": "Text",
  "mandatory_depends_on": "eval:doc.status == \"Rejected\""
}
```

If `status` is "Rejected" and `rejection_reason` is empty, save will fail with a validation error.

### Supported Expressions

| Expression | Meaning |
|-----------|---------|
| `fieldname` | Truthy check (non-empty, non-zero, non-null) |
| `eval:doc.fieldname` | Same, with Frappe-style prefix |
| `eval:doc.status == "Active"` | Equality check |
| `eval:doc.type != "Draft"` | Inequality check |

### depends_on — Conditional Field Visibility

Controls whether a field is shown in the form (client-side only):

```json
{
  "fieldname": "shipping_address",
  "label": "Shipping Address",
  "fieldtype": "Text",
  "depends_on": "eval:doc.requires_shipping"
}
```

When `requires_shipping` is falsy, the field is hidden.

### read_only_depends_on — Conditional Read-Only

```json
{
  "fieldname": "amount",
  "label": "Amount",
  "fieldtype": "Currency",
  "read_only_depends_on": "eval:doc.is_locked"
}
```
