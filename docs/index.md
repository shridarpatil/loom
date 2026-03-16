# Loom Framework

A metadata-driven application framework written in Rust, inspired by Python's Frappe Framework.

Build full-stack business applications by defining DocTypes — data models with built-in CRUD, permissions, workflows, and auto-generated REST APIs + admin UI.

**No end user needs a Rust toolchain.** Apps are built using JSON DocType definitions and embedded Rhai scripts.

---

## Quick Start

```bash
# Create a new site
loom new-site mysite.localhost

# Start the server
loom serve

# Open the Desk UI
open http://localhost:8000
```

## Key Features

- **DocType Engine** — Define data models as JSON, get CRUD, REST API, and admin UI automatically
- **Core DocTypes** — Built-in User and Role DocTypes for authentication and access control
- **Rhai Scripting** — Write server-side hooks, validations, and API methods without compiling Rust
- **Client Scripts** — Add custom buttons and validation to forms and list views via JavaScript
- **Permission System** — Role-based access with field-level permissions and User Permission filtering
- **Background Queue** — Custom named queues via hooks.toml, concurrent workers, scheduled tasks
- **File Upload** — Built-in file upload API with sanitized filenames
- **Developer Mode** — File watcher for hot-reloading DocTypes, scripts, and client scripts on save
- **Fixture Export** — Export document records as JSON fixtures via CLI or hooks.toml declarations
- **Realtime** — WebSocket pub/sub for live document updates across tabs/users
- **Multi-tenant** — Each site has its own database and site_config.json
- **App System** — Install 3rd party apps without a compiler

## Architecture

```
Layer 3: Compiled Rust (framework core only)
Layer 2: Rhai Scripts (hooks, validations, hot-reloadable)
Layer 1: JSON DocTypes (data models — no compilation needed)
```

Most apps only use Layers 1 and 2.

*WASM plugins and email sending are planned for a future release.*
