# Loom

Metadata-driven application framework in Rust, inspired by [Frappe](https://frappeframework.com). Build full-stack business applications by defining DocTypes — data models with built-in CRUD, permissions, workflows, and auto-generated REST APIs + admin UI.

**No end user needs a Rust toolchain.** Apps are built using JSON DocType definitions, embedded Rhai scripts, and optional WASM plugins.

## Features

- **DocType Engine**
  Define data models as JSON — get database tables, REST API, and admin UI automatically. Child tables, submittable documents, naming rules, and field-level permissions.

- **Rhai Scripting**
  Write server-side hooks, validations, and API methods without compiling Rust. Hot-reloadable in developer mode.

- **Permission System**
  Three-layer security: role-based DocPerm, field-level permlevel, and user-based row filtering. Role Permission Manager for site-level overrides without modifying app code.

- **Background Queue**
  Named queues with priority ordering, automatic retries, and concurrent workers. Cron-like scheduler for recurring tasks.

- **Realtime**
  WebSocket pub/sub — document changes broadcast to all connected clients. List views and forms auto-refresh across tabs.

- **App System**
  Install 3rd party apps without a compiler. JSON DocTypes + Rhai scripts + fixtures. Developer mode auto-exports changes to files.

- **Activity Timeline**
  Audit trail on every document — tracks who changed what with before/after values. Comments support.

- **Dynamic Admin UI**
  Vue 3 frontend with dynamic forms, list views, DocType builder, role permission manager, workspace, and customization.

## Quick Start

### Docker

```bash
git clone https://github.com/shridarpatil/loom.git
cd loom
docker compose up -d
```

Go to `http://localhost:8000` and login with `Administrator` / `admin`

### From Source

```bash
# Prerequisites: Rust 1.75+, PostgreSQL 14+, Node.js 18+

# Build
cargo build --release
cd frontend && npm install && npx vite build && cd ..

# Setup
createdb loom_dev
./target/release/loom_cli new-site mysite.localhost --db-url postgres://localhost/loom_dev

# Run
./target/release/loom_cli serve --db-url postgres://localhost/loom_dev
```

## Creating an App

```bash
# Scaffold
loom new-app my_app

# Creates:
# apps/my_app/
# ├── loom_app.toml     — App metadata
# ├── hooks.toml        — Scheduled tasks, fixtures, queues
# ├── doctypes/         — DocType definitions (JSON + Rhai)
# ├── api/              — Whitelisted API methods
# ├── fixtures/         — Seed data
# └── scripts/          — Shared scripts

# Install
loom install-app my_app
```

## DocType Example

```json
{
  "name": "Todo",
  "module": "My App",
  "naming_rule": "autoincrement",
  "fields": [
    { "fieldname": "title", "label": "Title", "fieldtype": "Data", "reqd": true, "in_list_view": true },
    { "fieldname": "status", "label": "Status", "fieldtype": "Select", "options": "Open\nCompleted", "default": "Open" },
    { "fieldname": "due_date", "label": "Due Date", "fieldtype": "Date" }
  ],
  "permissions": [
    { "role": "All", "read": true, "write": true, "create": true, "delete": true }
  ]
}
```

## Rhai Script Example

```rhai
fn validate(doc) {
    if doc.due_date != () && doc.due_date < today() {
        throw("Due date cannot be in the past");
    }
    doc
}

fn on_submit(doc) {
    loom_enqueue("my_app.send_notification", #{
        title: doc.title,
    });
}
```

## CLI

```bash
loom serve                          # Start server + workers
loom migrate                        # Sync DocTypes → database
loom new-app my_app                 # Scaffold app
loom install-app my_app             # Install app into site
loom worker --queue long -c 4       # Start 4 workers for "long" queue
loom export-fixtures                # Export fixture data from hooks.toml
loom console                        # Interactive Rhai REPL
```

## Tech Stack

| Component | Choice |
|-----------|--------|
| Language | Rust |
| HTTP | Axum |
| Database | PostgreSQL |
| Scripting | Rhai |
| Frontend | Vue 3 + TypeScript + Tailwind |
| Queue | PostgreSQL-backed (named queues + priority) |
| Realtime | WebSocket (Tokio broadcast) |

## Documentation

[https://shridarpatil.github.io/loom/](https://shridarpatil.github.io/loom/)

## License

MIT
