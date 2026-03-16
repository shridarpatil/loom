# Permission System

Loom has a three-layer permission system.

## Layer 1: DocPerm (Role-Based)

Defined in the DocType JSON or via the Role Permission Manager.

```json
{
  "permissions": [
    { "role": "HR Manager", "permlevel": 0, "read": true, "write": true, "create": true, "delete": true },
    { "role": "Employee", "permlevel": 0, "read": true, "if_owner": true }
  ]
}
```

### Permission Types

| Type | Description |
|------|-------------|
| `read` | View documents |
| `write` | Edit documents |
| `create` | Create new documents |
| `delete` | Delete documents |
| `submit` | Submit draft documents |
| `cancel` | Cancel submitted documents |
| `report` | View reports |
| `export` | Export data |

### `if_owner`

When `true`, the user can only access documents they created.

## Layer 2: Permlevel (Field-Level)

Fields can have a `permlevel` (0-9). A user needs a permission rule at that level to see/edit the field.

```json
{ "fieldname": "salary", "fieldtype": "Currency", "permlevel": 1 }
```

With permissions:

```json
[
  { "role": "Employee", "permlevel": 0, "read": true, "write": true },
  { "role": "HR Manager", "permlevel": 1, "read": true, "write": true }
]
```

- Employees see level-0 fields (name, department, etc.) but not salary
- HR Managers see both level-0 and level-1 fields
- Having read at any level implies level-0 visibility (base fields are always shown)
- Write is independent per level — level-1 write only affects level-1 fields

## Layer 3: User Permission (Link-Based Filtering)

Restricts which documents a user can see based on Link field values.

Example: "User X can only see documents where `company = 'Acme Corp'`"

Stored in the `__user_permission` table. Applied automatically to `get_list` queries — users only see documents matching their allowed values.

## Role Permission Manager

Site admins can override permissions without modifying the app's DocType definition.

Navigate to `/app/role-permission-manager` or click "Permissions" in the sidebar.

- **By DocType**: Select a DocType, add/edit permission overrides
- **By Role**: Select a role, see its permissions across all DocTypes

Overrides are stored separately from the app. Use "Reset to Defaults" to revert.

### How Overrides Work

- Defaults come from the DocType JSON (shipped with the app)
- Overrides are merged on top: matching `role + permlevel` pairs are replaced, new rules are added
- Default rules not touched by overrides remain in effect
- An empty override set means "use defaults"
