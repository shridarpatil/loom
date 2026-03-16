# Working with Child Tables

Child tables store rows that belong to a parent document — like line items on an invoice.

## Create the Child DocType

Using the DocType Builder (`/app/DocType/new`):

1. Enter a name (e.g., "Invoice Item")
2. Check **Child Table**
3. Add fields: item, qty, rate, amount
4. Save

Or as JSON:

```json
{
  "name": "Invoice Item",
  "module": "Accounting",
  "is_child_table": true,
  "fields": [
    { "fieldname": "item", "label": "Item", "fieldtype": "Data", "reqd": true },
    { "fieldname": "qty", "label": "Qty", "fieldtype": "Int", "reqd": true },
    { "fieldname": "rate", "label": "Rate", "fieldtype": "Currency" },
    { "fieldname": "amount", "label": "Amount", "fieldtype": "Currency" }
  ]
}
```

## Add to Parent DocType

In the parent DocType, add a Table field with `options` set to the child DocType name:

```json
{
  "fieldname": "items",
  "label": "Items",
  "fieldtype": "Table",
  "options": "Invoice Item"
}
```

## How It Works

When saving a parent document with child rows:

**Insert**: Parent is saved first, then each child row is inserted with `parent`, `parentfield`, `parenttype`, and `idx` set automatically.

**Update**: Existing child rows are deleted and re-inserted (full replace). This ensures idx ordering stays correct.

**Delete**: All child rows are deleted before the parent.

## Nested Child Tables

Child DocTypes can themselves contain Table fields. For example:

```
Invoice
  └── items (Table → Invoice Item)
       └── taxes (Table → Item Tax Detail)
```

The framework handles this recursively — grandchildren are inserted/deleted along with their parent rows.

## API

When fetching a document, child rows are included as arrays:

```json
{
  "data": {
    "id": "INV-001",
    "items": [
      { "id": "INV-001-items-1", "item": "Widget", "qty": 2, "rate": 100, "idx": 1 },
      { "id": "INV-001-items-2", "item": "Gadget", "qty": 1, "rate": 250, "idx": 2 }
    ]
  }
}
```

When creating/updating, send the child rows as arrays:

```bash
curl -X POST http://localhost:8000/api/resource/Invoice \
  -H "Content-Type: application/json" \
  -d '{
    "customer": "Acme Corp",
    "items": [
      { "item": "Widget", "qty": 2, "rate": 100 },
      { "item": "Gadget", "qty": 1, "rate": 250 }
    ]
  }'
```
