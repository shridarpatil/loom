# App System

Apps are the primary way to extend Loom. An app is a directory (or git repo) containing JSON DocType definitions, Rhai scripts, and fixtures.

**No Rust toolchain is required to develop or install apps.**

## App Structure

```
my_app/
├── loom_app.toml              # App metadata
├── doctypes/                  # DocType definitions
│   ├── employee/
│   │   ├── employee.json      # DocType meta
│   │   ├── employee.rhai      # Server script
│   │   └── employee.client.js # Client script (optional)
│   └── leave_type/
│       └── leave_type.json
├── api/                       # Whitelisted API methods
│   └── get_leave_balance.rhai
├── hooks.toml                 # Scheduled tasks, doc events, queues
├── fixtures/                  # Seed data
│   └── leave_type.json
├── plugins/                   # Reserved for future WASM plugins
└── frontend/                  # Optional frontend extensions
    └── src/
```

## loom_app.toml

```toml
[app]
name = "my_app"
version = "1.0.0"
title = "My Application"
description = "A custom Loom app"
modules = ["HR", "Payroll"]
```

## hooks.toml

```toml
# Scheduled tasks
[[scheduler]]
cron = "0 */6 * * *"
method = "my_app.sync_data"

# Document events
[[doc_events]]
doctype = "Employee"
event = "on_update"
method = "scripts/on_employee_update.rhai"

# Custom queues
[queues]
names = ["default", "long", "critical"]

# Fixture declarations (for loom export-fixtures)
[[fixtures]]
doctype = "Role"

[[fixtures]]
doctype = "Leave Type"
filters = { module = "HR" }

# Plugin pages (custom frontend routes)
[[pages]]
route = "/app/org-chart"
label = "Org Chart"
component = "OrgChart"
```

## Installing an App

```bash
# From a git repo
loom get-app https://github.com/someone/my_app

# From a local directory (already cloned)
# Just place it in the apps/ directory

# Install into the site
loom --site mysite.localhost install-app my_app
```

### What happens on `install-app`:

1. Reads `loom_app.toml` → registers the app
2. Loads all `doctypes/*.json` → inserts into DocType registry
3. Loads all `*.rhai` scripts → stores in `__script` table
4. Loads `*.client.js` → stores in `__customization` table
5. Runs auto-migration → creates/alters database tables
6. Loads `fixtures/*.json` → inserts seed data
7. Builds frontend extensions if present

## Client Scripts

Client scripts add client-side behavior to DocTypes — custom buttons, validation, and field change handlers. Place a `.client.js` file next to the DocType JSON:

```
my_app/doctypes/todo/
├── todo.json
├── todo.rhai          # Server script
└── todo.client.js     # Client script
```

### Example: todo.client.js

```javascript
// Validation — return an error string to block save
loom.validate = function(doc) {
  if (doc.due_date && new Date(doc.due_date) < new Date().setHours(0,0,0,0)) {
    return "Due Date cannot be in the past";
  }
};

// Custom button on the form view only
loom.add_button("Mark Complete", function(doc) {
  doc.status = "Completed";
}, { variant: "primary", view: "form" });

// Custom button on the list view only
loom.add_button("Close Selected", function(selectedRows) {
  if (!selectedRows || selectedRows.length === 0) {
    alert("Select rows first");
    return;
  }
  alert("Closing " + selectedRows.length + " todo(s)");
}, { view: "list" });
```

See [Scripting > Client Scripts](scripting.md#client-scripts) for the full API reference.

## Developer Mode

When `developer_mode` is enabled in `site_config.json`, Loom watches `apps/` and `core_doctypes/` for file changes:

- Saving a `.json` DocType file reloads and migrates it instantly
- Saving a `.rhai` script reloads it into the hook runner
- Saving a `.client.js` file updates the `__customization` table

This means you can edit files in your editor and see changes immediately without restarting the server.

## Customizing 3rd Party Apps

You can override permissions and field properties of installed apps without modifying the app's files:

- **Permissions**: Use the Role Permission Manager (`/app/role-permission-manager`)
- **Field properties**: Use Customize Form (`/app/customize-form/DocTypeName`)
- **Client scripts**: Add via Customize Form

Overrides are stored in `__customization` and persist across app updates.
