# Realtime

Loom uses WebSocket for real-time communication between the server and connected clients.

## How It Works

1. The frontend connects to `/ws` on page load
2. When any document is created, updated, deleted, submitted, or cancelled, the server broadcasts a `doc_update` event
3. Connected clients receive the event and auto-refresh

## Events

### `doc_update`

Published on every document mutation.

```json
{
  "event": "doc_update",
  "data": {
    "doctype": "Todo",
    "name": "TODO-001",
    "action": "updated"
  }
}
```

Actions: `created`, `updated`, `deleted`, `submitted`, `cancelled`

## Frontend Behavior

- **ListView**: Auto-refreshes when any document of the displayed DocType changes
- **Form**: Auto-reloads when the currently viewed document changes (only if no unsaved edits)
- **Connection**: Auto-reconnects every 5 seconds on disconnect

## Client API

```typescript
import { socket } from "@/utils/socket";

// Listen for events
socket.on("doc_update", (data) => {
  console.log(data.doctype, data.name, data.action);
});

// Subscribe to a specific document
socket.send("subscribe", { doctype: "Todo", name: "TODO-001" });

// Unsubscribe
socket.send("unsubscribe", { doctype: "Todo", name: "TODO-001" });
```

## Architecture

```
User A saves doc
    ↓
controller.rs → DB write
    ↓
resource.rs → realtime.publish_doc_update()
    ↓
RealtimeHub (broadcast channel)
    ↓
All WebSocket connections receive the event
    ↓
User B's ListView/Form auto-refreshes
```

The broadcast uses Tokio's `broadcast::channel` — efficient fan-out to all connected clients with no polling.
