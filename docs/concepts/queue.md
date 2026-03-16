# Background Queue

Loom has a built-in PostgreSQL-backed job queue with named queues, priority ordering, and automatic retries.

## How It Works

1. Jobs are enqueued into the `__job_queue` table
2. Workers poll their assigned queue and execute jobs as Rhai scripts
3. Failed jobs are retried (up to `max_retries`)
4. Jobs are picked by priority (highest first), then FIFO within same priority

## Enqueuing Jobs

### From Rhai Scripts

```rhai
// Default queue, default priority
loom_enqueue("my_app.send_email", #{ to: "user@example.com" });

// Named queue with priority (higher = runs first)
loom_enqueue("my_app.generate_report", #{ report_id: "RPT-001" }, #{
    queue: "long",
    priority: 5,
    max_retries: 5,
});
```

### Job Script

The enqueued method maps to a Rhai file: `my_app.send_email` → `apps/my_app/api/send_email.rhai`

```rhai
fn main(params, loom) {
    let to = params.to;
    log("Sending email to " + to);
    // ... email logic
}
```

## Named Queues

Queues are just string names. Any name works — workers process jobs from their assigned queue.

### Declaring Custom Queues

Apps declare their queues in `hooks.toml`:

```toml
[queues]
names = ["default", "long", "critical"]
```

`loom serve` reads `hooks.toml` from every installed app and auto-spawns one worker per declared queue. The `default` queue is always present even if not declared.

### Common Queue Strategy

| Queue | Use Case |
|-------|----------|
| `default` | Standard tasks (email, notifications) |
| `short` | Fast tasks (< 30s) |
| `long` | Heavy tasks (reports, data imports) |
| `critical` | Must-run tasks (payment processing) |

## Workers

### Auto-started by `loom serve`

Workers are spawned automatically for every queue declared in `hooks.toml`.

### Manual Workers

Start additional workers for scaling with `loom worker`:

```bash
# One worker for the "long" queue
loom worker --queue long

# Four concurrent workers for "critical"
loom worker --queue critical --concurrency 4

# Specify site (reads db credentials from site_config.json)
loom worker --site mysite.localhost --queue default

# Or override with explicit db URL
loom worker --queue default --db-url postgres://db-host/mysite
```

Workers can run on separate machines — they just need access to the same PostgreSQL database.

## Priority

Higher priority numbers run first. Default is `0`.

```rhai
// Priority 10 runs before priority 0
loom_enqueue("my_app.urgent_task", #{}, #{ priority: 10 });
loom_enqueue("my_app.normal_task", #{}, #{ priority: 0 });
```

## Retries

Jobs that fail are automatically re-queued up to `max_retries` times (default: 3). After all retries are exhausted, the job status is set to `failed` with the error message stored.

## Scheduled Tasks

Define cron-like schedules in `hooks.toml`:

```toml
[[scheduler]]
cron = "0 */6 * * *"
method = "my_app.sync_data"

[[scheduler]]
cron = "0 0 * * *"
method = "my_app.daily_cleanup"
```

Cron format: `minute hour day month weekday`

| Pattern | Meaning |
|---------|---------|
| `*` | Every |
| `*/N` | Every N |
| `N` | Exactly N |
| `N,M` | N or M |

The scheduler checks every minute and enqueues matching tasks on the `default` queue.

## Fixture Export via hooks.toml

Apps can declare which DocTypes should be exportable as fixtures in `hooks.toml`:

```toml
[[fixtures]]
doctype = "Role"

[[fixtures]]
doctype = "Leave Type"
filters = { module = "HR" }
```

Run `loom export-fixtures` to export all declared fixtures across all apps, or export a single DocType:

```bash
# Export all fixtures declared in hooks.toml
loom export-fixtures --site mysite.localhost

# Export a specific DocType with filters
loom export-fixtures --doctype Role --filters '{"module":"HR"}'
```

Exported files are written to `apps/{app}/fixtures/{doctype}.json`.
