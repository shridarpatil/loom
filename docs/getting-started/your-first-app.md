# Your First App

Apps in Loom are directories containing JSON DocType definitions, Rhai scripts, and optional WASM plugins. **No Rust toolchain is needed to develop or install apps.**

## Scaffold an App

```bash
loom new-app todo_app
```

This creates:

```
todo_app/
├── loom_app.toml         # App metadata
├── doctypes/             # DocType definitions
├── api/                  # Whitelisted API methods
├── scripts/              # Shared Rhai scripts
├── fixtures/             # Seed data
├── hooks.toml            # App-level hooks
└── frontend/             # Optional frontend extensions
```

## Create a DocType

### Option 1: JSON file

Create `todo_app/doctypes/todo/todo.json`:

```json
{
  "name": "Todo",
  "module": "Todo App",
  "naming_rule": "autoincrement",
  "fields": [
    {
      "fieldname": "title",
      "label": "Title",
      "fieldtype": "Data",
      "reqd": true,
      "in_list_view": true
    },
    {
      "fieldname": "status",
      "label": "Status",
      "fieldtype": "Select",
      "options": "Open\nIn Progress\nCompleted",
      "default": "Open",
      "in_list_view": true,
      "in_standard_filter": true
    },
    {
      "fieldname": "priority",
      "label": "Priority",
      "fieldtype": "Select",
      "options": "Low\nMedium\nHigh",
      "default": "Medium",
      "in_list_view": true
    },
    {
      "fieldname": "description",
      "label": "Description",
      "fieldtype": "Text"
    },
    {
      "fieldname": "due_date",
      "label": "Due Date",
      "fieldtype": "Date"
    }
  ],
  "permissions": [
    {
      "role": "All",
      "read": true,
      "write": true,
      "create": true,
      "delete": true
    }
  ]
}
```

### Option 2: Desk UI

Navigate to `/app/DocType/new` in the browser and use the DocType Builder.

## Install the App

```bash
loom --site mysite.localhost install-app todo_app
```

This loads the JSON, creates the database table, and registers the DocType. Navigate to `/app/Todo` to see your list view, or `/app/Todo/new` to create a record.

## Add a Script

Create `todo_app/doctypes/todo/todo.rhai`:

```rhai
fn validate(doc) {
    if doc.due_date != () && doc.due_date < today() {
        throw("Due date cannot be in the past");
    }
}

fn before_save(doc) {
    if doc.status == "Completed" && doc.completed_on == () {
        doc.completed_on = today();
    }
    doc
}
```

Re-install the app or restart the server to load the script.

## Add an API Method

Create `todo_app/api/get_open_count.rhai`:

```rhai
fn main(params, loom) {
    let count = loom_db_count("Todo", #{ status: "Open" });
    #{ count: count }
}
```

Call it:

```bash
curl -X POST http://localhost:8000/api/method/todo_app.get_open_count \
  -H "Content-Type: application/json" \
  -d '{}'
```
