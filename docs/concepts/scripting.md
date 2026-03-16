# Rhai Scripting

Rhai is the embedded scripting language for hooks, validations, and API methods. It's sandboxed, fast, and requires no external runtime.

## DocType Scripts

Place a `.rhai` file next to the DocType JSON:

```
my_app/doctypes/employee/
├── employee.json
└── employee.rhai
```

### Hook Events

```rhai
fn before_insert(doc) {
    // Runs before a new document is inserted
    doc
}

fn validate(doc) {
    // Runs before save (insert or update)
    if doc.email == "" {
        throw("Email is required");
    }
    doc
}

fn before_save(doc) {
    // Runs before DB write
    doc.full_name = doc.first_name + " " + doc.last_name;
    doc
}

fn on_update(doc) {
    // Runs after update
}

fn on_submit(doc) {
    // Runs after submit (docstatus 0 → 1)
}

fn before_cancel(doc) {
    // Runs before cancel
}

fn on_trash(doc) {
    // Runs before delete
}
```

### Execution Order

**Insert:** `before_insert` → `validate` → `before_save` → DB INSERT → `after_insert` → `after_save`

**Update:** `before_save` → `validate` → DB UPDATE → `on_update` → `after_save`

**Submit:** `before_submit` → `validate` → DB UPDATE → `on_submit`

**Cancel:** `before_cancel` → DB UPDATE → `on_cancel`

**Delete:** `on_trash` → DB DELETE

## Loom API

Available in all scripts:

### Database

```rhai
let doc = loom_db_get_doc("Employee", "EMP-001");

let name = loom_db_get_value("Employee", #{ department: "Engineering" }, "employee_name");

let employees = loom_db_get_all("Employee", #{
    filters: #{ status: "Active" },
    fields: ["employee_name", "department"],
    order_by: "creation desc",
    limit: 20,
});

loom_db_set_value("Employee", "EMP-001", "status", "Inactive");

loom_db_add_value("Leave Allocation", "LA-001", "used_leaves", 1.0);

let exists = loom_db_exists("Employee", #{ email: "john@example.com" });

let count = loom_db_count("Employee", #{ status: "Active" });

let new_doc = loom_db_insert(#{
    doctype: "Todo",
    title: "Follow up",
    status: "Open",
});

loom_db_delete("Todo", "TODO-123");

// Read-only raw SQL
let results = loom_db_sql("SELECT id, title FROM \"tabTodo\" WHERE status = $1", ["Open"]);
```

### Session

```rhai
let user = loom_session_user();       // "john@example.com"
let roles = loom_session_roles();     // ["All", "Employee"]
```

### Permissions

```rhai
let can_read = loom_has_permission("Employee", "read");  // true/false
loom_check_permission("Employee", "write");               // throws if denied
```

### Utilities

```rhai
throw("Validation error message");
log("Debug message");
msgprint("Info message for the user");

let today_str = today();                    // "2026-03-15"
let now_str = now();                        // ISO 8601 datetime
let days = date_diff("2026-03-15", "2026-03-01");  // 14
let next_week = loom_add_days("2026-03-15", 7);    // "2026-03-22"
let next_month = loom_add_months("2026-03-15", 1); // "2026-04-15"
```

### Background Jobs

```rhai
// Default queue
loom_enqueue("my_app.send_email", #{ to: "user@example.com" });

// Named queue with priority
loom_enqueue("my_app.heavy_report", #{ id: "RPT-001" }, #{
    queue: "long",
    priority: 5,
});
```

### Call Other Methods

```rhai
let result = loom_call("my_app.get_balance", #{ employee: "EMP-001" });
```

## Whitelisted API Methods

Place scripts in `api/`:

```
my_app/api/get_leave_balance.rhai
```

```rhai
fn main(params, loom) {
    loom_check_permission("Leave Application", "read");

    let balance = loom_db_get_value("Leave Allocation", #{
        employee: params.employee,
    }, "remaining_leaves");

    #{ balance: balance }
}
```

Call via HTTP:

```bash
POST /api/method/my_app.get_leave_balance
Content-Type: application/json

{"employee": "EMP-001"}
```

## Sandbox Limits

Scripts run in a sandboxed environment:

| Limit | Value |
|-------|-------|
| Max expression depth | 64 |
| Max call stack depth | 32 |
| Max operations | 100,000 |
| Max string size | 1 MB |
| Max array size | 10,000 |
| Max map size | 10,000 |
| No filesystem access | — |
| No network access | — |
