# CLI Reference

## `loom serve`

Start the web server with background workers.

```bash
loom serve [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-p, --port` | 8000 | Port to listen on |
| `--host` | 0.0.0.0 | Host to bind to |
| `--db-url` | `$DATABASE_URL` | PostgreSQL connection string |
| `--apps-dir` | `apps` | Path to apps directory |
| `--frontend-dir` | `frontend/dist` | Path to built frontend |

Auto-starts one worker per queue declared in `hooks.toml`.

## `loom new-site`

Create a new site with system tables and admin user.

```bash
loom new-site SITE_NAME --db-url postgres://localhost/mydb
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

Install an app into the site (loads DocTypes, scripts, fixtures, runs migrations).

```bash
loom --site mysite.localhost install-app my_app
```

## `loom migrate`

Sync DocType definitions to database schema (creates/alters tables).

```bash
loom migrate --db-url postgres://localhost/mydb
```

## `loom console`

Interactive Rhai REPL with database access.

```bash
loom console --db-url postgres://localhost/mydb
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
| `--db-url` | `$DATABASE_URL` | PostgreSQL connection string |

Examples:

```bash
# Process the "long" queue with 4 workers
loom worker --queue long --concurrency 4

# Process a custom queue
loom worker --queue critical
```

Workers can run on separate machines — they just need access to the same database.
