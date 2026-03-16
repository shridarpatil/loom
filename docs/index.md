# Loom Framework

A metadata-driven application framework written in Rust, inspired by Python's Frappe Framework.

Build full-stack business applications by defining DocTypes — data models with built-in CRUD, permissions, workflows, and auto-generated REST APIs + admin UI.

**No end user needs a Rust toolchain.** Apps are built using JSON DocType definitions, embedded Rhai scripts, and optional WASM plugins.

---

## Quick Start

```bash
# Create a new site
loom new-site mysite.localhost --db-url postgres://localhost/mysite

# Start the server
loom serve

# Open the Desk UI
open http://localhost:8000
```

## Key Features

- **DocType Engine** — Define data models as JSON, get CRUD, REST API, and admin UI automatically
- **Rhai Scripting** — Write hooks, validations, and API methods without compiling Rust
- **Permission System** — Role-based access with field-level permissions and User Permission filtering
- **Background Queue** — Named queues with priority, concurrent workers, scheduled tasks
- **Realtime** — WebSocket pub/sub for live document updates across tabs/users
- **Multi-tenant** — Each site has its own database
- **App System** — Install 3rd party apps without a compiler

## Architecture

```
Layer 4: Compiled Rust (framework core only)
Layer 3: WASM Plugins (complex app logic, pre-compiled)
Layer 2: Rhai Scripts (hooks, validations, hot-reloadable)
Layer 1: JSON DocTypes (data models — no compilation needed)
```

Most apps only use Layers 1 and 2.
