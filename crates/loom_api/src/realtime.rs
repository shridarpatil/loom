use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use serde_json::{json, Value};
use tokio::sync::{broadcast, RwLock};

use crate::server::AppState;

/// Shared realtime state — manages broadcast channels per doctype:name.
#[derive(Debug, Clone)]
pub struct RealtimeHub {
    /// Global broadcast for all events (doctype changes, progress, etc.)
    global_tx: broadcast::Sender<RealtimeEvent>,
    /// Per-document subscriptions: "DocType:name" → list of subscriber IDs
    subscriptions: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

#[derive(Debug, Clone)]
pub struct RealtimeEvent {
    pub event: String,
    pub data: Value,
}

impl RealtimeHub {
    pub fn new() -> Self {
        let (global_tx, _) = broadcast::channel(1024);
        Self {
            global_tx,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Publish an event to all connected clients.
    pub fn publish(&self, event: &str, data: Value) {
        let _ = self.global_tx.send(RealtimeEvent {
            event: event.to_string(),
            data,
        });
    }

    /// Publish a document change event.
    pub fn publish_doc_update(&self, doctype: &str, name: &str, action: &str) {
        self.publish(
            "doc_update",
            json!({
                "doctype": doctype,
                "name": name,
                "action": action,
            }),
        );
    }

    /// Subscribe to the global broadcast.
    fn subscribe(&self) -> broadcast::Receiver<RealtimeEvent> {
        self.global_tx.subscribe()
    }

    fn next_conn_id(&self) -> u64 {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

/// GET /ws — WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let conn_id = state.realtime.next_conn_id();
    tracing::debug!("WebSocket connected: {}", conn_id);

    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.realtime.subscribe();

    use futures_util::{SinkExt, StreamExt};

    // Spawn a task to forward broadcast events to this client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let msg = json!({
                "event": event.event,
                "data": event.data,
            });
            if sender
                .send(Message::Text(msg.to_string().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Handle incoming messages from the client
    let hub = state.realtime.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                        let event = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("");
                        let data = parsed.get("data").cloned().unwrap_or(Value::Null);

                        match event {
                            "subscribe" => {
                                // Client wants to subscribe to a doc: { doctype, name }
                                let key = format!(
                                    "{}:{}",
                                    data.get("doctype").and_then(|v| v.as_str()).unwrap_or(""),
                                    data.get("name").and_then(|v| v.as_str()).unwrap_or("")
                                );
                                hub.subscriptions
                                    .write()
                                    .await
                                    .entry(key)
                                    .or_default()
                                    .push(conn_id);
                            }
                            "unsubscribe" => {
                                let key = format!(
                                    "{}:{}",
                                    data.get("doctype").and_then(|v| v.as_str()).unwrap_or(""),
                                    data.get("name").and_then(|v| v.as_str()).unwrap_or("")
                                );
                                if let Some(subs) = hub.subscriptions.write().await.get_mut(&key) {
                                    subs.retain(|id| *id != conn_id);
                                }
                            }
                            _ => {
                                tracing::debug!("WS received unknown event: {}", event);
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish (client disconnect)
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // Cleanup subscriptions for this connection
    let mut subs = state.realtime.subscriptions.write().await;
    for subscribers in subs.values_mut() {
        subscribers.retain(|id| *id != conn_id);
    }

    tracing::debug!("WebSocket disconnected: {}", conn_id);
}
