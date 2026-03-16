# DocTypes

A DocType is a data model definition. When you create a DocType, Loom automatically generates:

- A database table
- REST API endpoints (CRUD)
- An admin UI (form + list view)
- Permission rules

## Definition Format

```json
{
  "name": "Employee",
  "module": "HR",
  "is_submittable": false,
  "naming_rule": "autoincrement",
  "title_field": "employee_name",
  "sort_field": "creation",
  "sort_order": "desc",
  "fields": [...],
  "permissions": [...]
}
```

## Field Types

| Type | SQL Type | Description |
|------|----------|-------------|
| Data | VARCHAR(140) | Short text |
| Link | VARCHAR(140) | Reference to another DocType |
| Select | VARCHAR(140) | Dropdown with options |
| Int | BIGINT | Integer |
| Float | DOUBLE PRECISION | Decimal number |
| Currency | NUMERIC(18,6) | Money |
| Check | BOOLEAN | Yes/No |
| Date | DATE | Date |
| Datetime | TIMESTAMP | Date and time |
| Text | TEXT | Multi-line text |
| Table | — | Child table (rows stored in child DocType's table) |
| SectionBreak | — | Layout: section divider |
| ColumnBreak | — | Layout: column divider |

## Naming Rules

| Rule | Description |
|------|-------------|
| `autoincrement` | Database sequence (1, 2, 3...) |
| `hash` | Random 10-char hash |
| `series` | Pattern like `HR-EMP-.#####` |
| `by_fieldname` | Use a field's value as the name |
| `prompt` | User provides the name |
| `expression` | Format string with doc fields |

## Standard Fields

Every DocType automatically gets these fields (never define them in JSON):

| Field | Type | Description |
|-------|------|-------------|
| `id` | VARCHAR(140) | Primary key |
| `owner` | VARCHAR(140) | Creator |
| `creation` | TIMESTAMP | Created at |
| `modified` | TIMESTAMP | Last modified |
| `modified_by` | VARCHAR(140) | Last modified by |
| `docstatus` | SMALLINT | 0=Draft, 1=Submitted, 2=Cancelled |

## Child Tables

A child DocType stores rows that belong to a parent. Create it with `is_child_table: true` (or check "Child Table" in the DocType Builder).

```json
{
  "name": "Invoice Item",
  "module": "Accounting",
  "is_child_table": true,
  "fields": [
    { "fieldname": "item", "label": "Item", "fieldtype": "Data", "reqd": true },
    { "fieldname": "qty", "label": "Qty", "fieldtype": "Int", "reqd": true },
    { "fieldname": "rate", "label": "Rate", "fieldtype": "Currency" },
    { "fieldname": "amount", "label": "Amount", "fieldtype": "Currency" }
  ]
}
```

In the parent DocType, add a Table field:

```json
{
  "fieldname": "items",
  "label": "Items",
  "fieldtype": "Table",
  "options": "Invoice Item"
}
```

Nested child tables are supported — a child DocType can itself contain Table fields.

## Submittable Documents

Set `is_submittable: true` for documents that follow a Draft → Submitted → Cancelled workflow.

- **Draft** (docstatus=0): Editable
- **Submitted** (docstatus=1): Locked, cannot be edited
- **Cancelled** (docstatus=2): Reversed

Requires `submit` and `cancel` permissions on the role.
