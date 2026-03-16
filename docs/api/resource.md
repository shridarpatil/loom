# Resource API

CRUD endpoints auto-generated for every DocType.

## List Documents

```
GET /api/resource/{doctype}
```

Query parameters:

| Param | Type | Description |
|-------|------|-------------|
| `filters` | JSON string | `[["status","=","Open"]]` or `{"status":"Open"}` |
| `fields` | JSON array or CSV | `["id","title","status"]` |
| `order_by` | string | `"modified desc"` |
| `limit` | integer | Max rows (default: 20) |
| `offset` | integer | Skip rows |
| `search_term` | string | Full-text search on name + title + search fields |

Example:

```bash
curl 'http://localhost:8000/api/resource/Todo?filters=[["status","=","Open"]]&limit=10'
```

Response:

```json
{
  "data": [
    { "id": "1", "title": "Buy groceries", "status": "Open" },
    { "id": "2", "title": "Call dentist", "status": "Open" }
  ]
}
```

## Get Document

```
GET /api/resource/{doctype}/{name}
```

```json
{
  "data": {
    "id": "1",
    "title": "Buy groceries",
    "status": "Open",
    "owner": "Administrator",
    "creation": "2026-03-15 10:00:00",
    "modified": "2026-03-15 10:00:00"
  }
}
```

## Create Document

```
POST /api/resource/{doctype}
Content-Type: application/json

{ "title": "New task", "status": "Open" }
```

Returns `201 Created` with the full document.

## Update Document

```
PUT /api/resource/{doctype}/{name}
Content-Type: application/json

{ "status": "Completed" }
```

Only send the fields you want to change.

## Delete Document

```
DELETE /api/resource/{doctype}/{name}
```

Returns `{ "message": "ok" }`.

## Submit Document

```
POST /api/resource/{doctype}/{name}/submit
```

Changes `docstatus` from 0 to 1. Only works on submittable DocTypes.

## Cancel Document

```
POST /api/resource/{doctype}/{name}/cancel
```

Changes `docstatus` from 1 to 2.

## Filter Operators

When using array-format filters `[field, operator, value]`:

| Operator | Description |
|----------|-------------|
| `=` | Equal |
| `!=` | Not equal |
| `>`, `>=`, `<`, `<=` | Comparison |
| `like` | SQL LIKE (`%pattern%`) |
| `not like` | SQL NOT LIKE |
| `in` | In array: `["status", "in", ["Open", "Draft"]]` |
| `not in` | Not in array |
| `between` | Range: `["date", "between", ["2026-01-01", "2026-12-31"]]` |
| `is null` | Is NULL |
| `is not null` | Is not NULL |
