use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::application::event_bus::DeviceEventBus;

/// WebSocket handler state.
#[derive(Clone)]
pub struct WsState {
    pub event_bus: DeviceEventBus,
}

/// Query parameters for WebSocket subscription filtering.
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Filter events by product_key. If omitted, receive all events.
    pub product_key: Option<String>,
    /// Filter events by device_id. If omitted, receive all events for the product.
    pub device_id: Option<String>,
}

/// WebSocket upgrade handler.
///
/// GET /api/v1/ws?product_key=xxx&device_id=yyy
///
/// Upgrades the HTTP connection to WebSocket and streams device events as JSON.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<WsState>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    info!(
        product_key = ?query.product_key,
        device_id = ?query.device_id,
        "WebSocket connection upgrading"
    );
    ws.on_upgrade(move |socket| handle_socket(socket, state.event_bus, query))
}

async fn handle_socket(socket: WebSocket, event_bus: DeviceEventBus, query: WsQuery) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = event_bus.subscribe();

    let filter_pk = query.product_key;
    let filter_did = query.device_id;

    info!(
        filter_pk = ?filter_pk,
        filter_did = ?filter_did,
        subscribers = event_bus.subscriber_count(),
        "WebSocket client connected"
    );

    // Spawn a task to forward broadcast events to the WebSocket client
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    // Apply filters
                    if let Some(ref pk) = filter_pk {
                        if event.product_key() != pk {
                            continue;
                        }
                    }
                    if let Some(ref did) = filter_did {
                        if event.device_id() != did {
                            continue;
                        }
                    }

                    // Serialize and send
                    match serde_json::to_string(&event) {
                        Ok(json) => {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                debug!("WebSocket send failed, client disconnected");
                                break;
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to serialize device event");
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(missed = n, "WebSocket client lagged, skipping messages");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("Event bus closed");
                    break;
                }
            }
        }
    });

    // Spawn a task to handle incoming WebSocket messages (ping/pong/close)
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => {
                    debug!("WebSocket client sent close frame");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    // axum handles pong automatically
                }
                Ok(_) => {
                    // Ignore other messages from client
                }
                Err(e) => {
                    debug!(error = %e, "WebSocket receive error");
                    break;
                }
            }
        }
    });

    // Wait for either task to finish, then abort the other
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    info!("WebSocket client disconnected");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_query_deserialize() {
        let query: WsQuery = serde_json::from_str(r#"{"product_key":"pk001"}"#).unwrap();
        assert_eq!(query.product_key.as_deref(), Some("pk001"));
        assert!(query.device_id.is_none());
    }

    #[test]
    fn test_ws_query_empty() {
        let query: WsQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert!(query.product_key.is_none());
        assert!(query.device_id.is_none());
    }
}
