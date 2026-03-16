# App System

Apps are the primary way to extend Loom. An app is a directory (or git repo) containing JSON, scripts, and optionally WASM plugins.

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
├── plugins/                   # Optional WASM plugins
│   └── payroll_engine.wasm
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
7. Copies `plugins/*.wasm` → site plugin directory

## Customizing 3rd Party Apps

You can override permissions and field properties of installed apps without modifying the app's files:

- **Permissions**: Use the Role Permission Manager (`/app/role-permission-manager`)
- **Field properties**: Use Customize Form (`/app/customize-form/DocTypeName`)
- **Client scripts**: Add via Customize Form

Overrides are stored in `__customization` and persist across app updates.
