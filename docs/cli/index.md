# CLI Reference

All commands accept a global `--site` flag to specify the target site. When omitted and only one site exists in `sites/`, it is auto-detected. Database credentials are read from `sites/{site}/site_config.json`, so `--db-url` is rarely needed.

```bash
loom [--site SITE_NAME] <command> [OPTIONS]
```

## `loom serve`

Start the web server with background workers.

```bash
loom serve [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-p, --port` | 8000 | Port to listen on |
| `--host` | 0.0.0.0 | Host to bind to |
| `--site` | (auto-detect) | Site name |
| `--db-url` | site_config.json | PostgreSQL connection string (override) |
| `--apps-dir` | `apps` | Path to apps directory |
| `--frontend-dir` | `frontend/dist` | Path to built frontend |

Auto-starts one worker per queue declared in `hooks.toml`. Enables file watching when `developer_mode` is true in site_config.json.

## `loom new-site`

Create a new site directory with site_config.json.

```bash
loom new-site SITE_NAME [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--db-name` | derived from site name | PostgreSQL database name |
| `--admin-password` | `admin` | Initial admin password |
| `--db-url` | — | Database URL (override) |

```bash
loom new-site mysite.localhost --db-name loom_dev
```

## `loom new-app`

Scaffold a new app directory.

```bash
loom new-app my_app
```

## `loom get-app`

Download an app from git, URL, or local archive.

```bash
loom get-app https://github.com/someone/my_app
loom get-app /path/to/my_app.tar.gz
```

## `loom install-app`

Install an app into the site. This runs the full `migrate` pipeline (DocTypes, scripts, client scripts, app configs) and then loads fixtures. There is no need to run `loom migrate` separately after install.

```bash
loom --site mysite.localhost install-app my_app
```

## `loom migrate`

Sync DocType definitions to database schema (creates/alters tables). Also handles:

- Core and app DocType loading and table migration
- Server scripts (`.rhai`) — loaded into `__script`
- Client scripts (`.client.js`) — loaded into `__customization`
- App configs from `hooks.toml` — workspace entries, dashboard widgets, queues, scheduler, pages
- Default user and role seeding

After pulling app changes from git, running `loom migrate` is all that's needed to apply them.

```bash
loom migrate [--site mysite.localhost]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--site` | (auto-detect) | Site name |
| `--db-url` | site_config.json | PostgreSQL connection string (override) |
| `--apps-dir` | `apps` | Path to apps directory |

## `loom console`

Interactive Rhai REPL with database access. Runs as Administrator with full permissions.

```bash
loom console [--site mysite.localhost]
```

```
loom> loom_db_count("Todo", #{})
42
loom> today()
"2026-03-15"
```

## `loom worker`

Start a standalone background job worker.

```bash
loom worker [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-q, --queue` | `default` | Queue name to process |
| `-c, --concurrency` | 1 | Number of concurrent workers |
| `--site` | (auto-detect) | Site name |
| `--db-url` | site_config.json | PostgreSQL connection string (override) |

Examples:

```bash
# Process the "long" queue with 4 workers
loom worker --queue long --concurrency 4

# Process a custom queue on a specific site
loom worker --site mysite.localhost --queue critical
```

Workers can run on separate machines — they just need access to the same database.

## `loom export-fixtures`

Export document records as fixture JSON files.

```bash
loom export-fixtures [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-d, --doctype` | — | DocType to export (omit to export all from hooks.toml) |
| `-f, --filters` | — | JSON filters, e.g. `'{"module":"HR"}'` |
| `-a, --app` | (auto-detect) | Target app directory name |
| `--site` | (auto-detect) | Site name |
| `--db-url` | site_config.json | PostgreSQL connection string (override) |

Examples:

```bash
# Export all fixtures declared in hooks.toml
loom export-fixtures

# Export a single DocType with filters
loom export-fixtures --doctype Role --filters '{"module":"Core"}'

# Export to a specific app
loom export-fixtures --doctype "Leave Type" --app loom_hr
```

Output is written to `apps/{app}/fixtures/{doctype}.json`.
