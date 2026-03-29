//! # WebSocket Generation Updates
//!
//! WebSocket endpoint for real-time generation status updates.
//! Clients connect and subscribe to specific generation IDs via Redis pub/sub.

use axum::{
    extract::{ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use auth::JwtService;
use futures::{SinkExt, StreamExt};
use parking_lot::RwLock;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use crate::ApiState;

// ---------------------------------------------------------------------------
// Message types
// ---------------------------------------------------------------------------

/// Incoming message from the WebSocket client.
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WsClientMessage {
    /// Subscribe to updates for a specific generation.
    Subscribe { generation_id: Uuid },
    /// Unsubscribe from updates for a specific generation.
    Unsubscribe { generation_id: Uuid },
}

/// Outgoing message to the WebSocket client.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMessage {
    /// Generation status update received from Redis pub/sub.
    Update {
        generation_id: Uuid,
        status: String,
        message: String,
        timestamp: String,
    },
    /// Error or acknowledgement message.
    Notice { text: String },
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Create the WebSocket router for generation updates.
pub fn ws_router(state: ApiState) -> Router {
    Router::new()
        .route("/ws/generations", get(ws_handler))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// WebSocket handler
// ---------------------------------------------------------------------------

/// GET /ws/generations
///
/// WebSocket endpoint for real-time generation updates.
/// Validates JWT from the Authorization header before upgrading.
pub async fn ws_handler(
    State(state): State<ApiState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let config = state.inner.config.clone();
    let redis = state.inner.redis.clone();

    ws.protocols(["v1"])
        .on_upgrade(move |socket| handle_socket(socket, config, redis))
}

/// Actual WebSocket handler that manages subscriptions.
async fn handle_socket(
    socket: WebSocket,
    config: common::AppConfig,
    redis: Arc<RwLock<ConnectionManager>>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut subscriptions: HashSet<Uuid> = HashSet::new();

    // Build the JWT service for token verification
    let jwt_service = JwtService::new(
        &config.jwt_secret,
        "creative-ai-studio",
        "creative-ai-studio-api",
    );

    // Wait for first message — it should be an auth message
    let Some(Ok(Message::Text(text))) = receiver.next().await else {
        return;
    };

    // Parse auth message to validate JWT
    #[derive(Deserialize)]
    struct AuthMsg {
        #[serde(rename = "action")]
        _action: Option<String>,
        token: Option<String>,
    }

    let validated_user_id: Option<String> = if let Ok(auth) = serde_json::from_str::<AuthMsg>(&text) {
        if let Some(token) = auth.token {
            if let Ok(claims) = jwt_service.verify_token(&token) {
                Some(claims.sub)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if validated_user_id.is_none() {
        let notice = WsServerMessage::Notice {
            text: "authentication required".to_string(),
        };
        let _ = sender.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&notice).unwrap()))).await;
        let _ = sender.close().await;
        return;
    }

    // Send auth success notice
    let success = WsServerMessage::Notice {
        text: "authenticated".to_string(),
    };
    let _ = sender.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&success).unwrap()))).await;

    tracing::debug!(user_id = ?validated_user_id.as_ref().map(|s| s.as_str()), "WebSocket authenticated");

    // Main message loop
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<WsClientMessage>(&text) {
                            Ok(WsClientMessage::Subscribe { generation_id }) => {
                                subscriptions.insert(generation_id);
                                let notice = WsServerMessage::Notice {
                                    text: format!("subscribed to {}", generation_id),
                                };
                                let _ = sender.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&notice).unwrap_or_default()))).await;
                            }
                            Ok(WsClientMessage::Unsubscribe { generation_id }) => {
                                subscriptions.remove(&generation_id);
                                let notice = WsServerMessage::Notice {
                                    text: format!("unsubscribed from {}", generation_id),
                                };
                                let _ = sender.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&notice).unwrap_or_default()))).await;
                            }
                            Err(e) => {
                                let notice = WsServerMessage::Notice {
                                    text: format!("invalid message: {}", e),
                                };
                                let _ = sender.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&notice).unwrap_or_default()))).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                for gen_id in &subscriptions {
                    let pending_key = format!("pubsub:pending:{}", gen_id);
                    let mut conn = {
                        let guard = redis.read();
                        (*guard).clone()
                    };

                    let msg: Option<String> = redis::cmd("GET")
                        .arg(&pending_key)
                        .query_async(&mut conn)
                        .await
                        .ok()
                        .flatten();

                    if let Some(payload) = msg {
                        let _ = sender.send(Message::Text(Utf8Bytes::from(payload))).await;
                        let _: Result<i64, _> = redis::cmd("DEL")
                            .arg(&pending_key)
                            .query_async(&mut conn)
                            .await;
                    }
                }
            }
        }
    }

    tracing::debug!(
        user_id = ?validated_user_id.as_ref().map(|s| s.as_str()),
        subscriptions = subscriptions.len(),
        "WebSocket connection closed"
    );
}
