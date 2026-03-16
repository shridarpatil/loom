# Installation

## Prerequisites

- **Rust** 1.75+ (only for building the framework itself)
- **PostgreSQL** 14+
- **Node.js** 18+ (for building the frontend)

## Build from Source

```bash
git clone https://github.com/anthropics/loom.git
cd loom

# Build the CLI
cargo build --release

# Build the frontend
cd frontend && npm install && npx vite build && cd ..

# The binary is at target/release/loom_cli
alias loom="./target/release/loom_cli"
```

## Create Your First Site

```bash
# Create a PostgreSQL database
createdb loom_dev

# Initialize the site
loom new-site mysite.localhost --db-name loom_dev

# Start the server
loom serve

# Open http://localhost:8000
# Login: Administrator / admin
```

## site_config.json

Each site has a `sites/{site_name}/site_config.json` that stores database credentials and site settings. Commands like `loom serve` and `loom migrate` read this automatically, so you don't need to pass `--db-url` every time.

```json
{
  "db_name": "loom_dev",
  "db_type": "postgres",
  "db_host": "localhost",
  "db_port": 5432,
  "db_user": "loom",
  "db_password": "loom",
  "developer_mode": true
}
```

| Field | Default | Description |
|-------|---------|-------------|
| `db_name` | `loom` | PostgreSQL database name |
| `db_host` | `localhost` | Database host |
| `db_port` | `5432` | Database port |
| `db_user` | `postgres` | Database user |
| `db_password` | `""` | Database password |
| `developer_mode` | `false` | Enable file watching and hot-reload (see below) |

### Developer Mode

Set `"developer_mode": true` in site_config.json to enable:

- **File watcher** — Automatically watches `apps/` and `core_doctypes/` for changes
- **Hot-reload DocTypes** — Saving a `.json` file re-registers the DocType and runs migrations
- **Hot-reload scripts** — Saving a `.rhai` file reloads it into the hook runner
- **Hot-reload client scripts** — Saving a `.client.js` file updates the `__customization` table

No server restart needed for any of these changes.

## Docker

```bash
docker-compose up -d
```

This starts PostgreSQL and the Loom server. The Desk UI is available at `http://localhost:8000`.
