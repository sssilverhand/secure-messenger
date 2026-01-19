//! WebSocket handler for real-time communication

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use chrono::DateTime;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::{
    models::*,
    AppState,
};

/// Parse datetime string to timestamp
fn parse_datetime_to_timestamp(s: &str) -> i64 {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp())
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<WsServerMessage>();

    let mut user_id: Option<String> = None;
    let mut device_id: Option<String> = None;

    // Task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<WsClientMessage>(&text) {
                    Ok(client_msg) => {
                        match client_msg {
                            WsClientMessage::Authenticate { token } => {
                                // Validate session
                                if let Ok(Some(session)) = state.storage.validate_session(&token).await {
                                    user_id = Some(session.user_id.clone());
                                    device_id = Some(session.device_id.clone());

                                    // Register connection
                                    state.ws_manager.register(
                                        &session.user_id,
                                        &session.device_id,
                                        tx.clone(),
                                    );

                                    // Send authenticated response
                                    let _ = tx.send(WsServerMessage::Authenticated {
                                        user_id: session.user_id.clone(),
                                        device_id: session.device_id.clone(),
                                    });

                                    // Deliver pending messages
                                    if let Ok(pending) = state.storage.get_pending_messages(
                                        &session.user_id,
                                        Some(&session.device_id),
                                    ).await {
                                        for pm in pending {
                                            let envelope = MessageEnvelope {
                                                message_id: pm.message_id,
                                                sender_id: pm.sender_id,
                                                recipient_id: pm.recipient_id,
                                                recipient_device_id: pm.recipient_device_id,
                                                encrypted_content: pm.encrypted_content,
                                                message_type: pm.message_type.into(),
                                                timestamp: parse_datetime_to_timestamp(&pm.created_at),
                                            };
                                            let _ = tx.send(WsServerMessage::Message(envelope));
                                        }
                                    }

                                    tracing::info!(
                                        "WebSocket authenticated: user={}, device={}",
                                        session.user_id,
                                        session.device_id
                                    );
                                } else {
                                    let _ = tx.send(WsServerMessage::Error {
                                        code: "AUTH_FAILED".to_string(),
                                        message: "Invalid or expired token".to_string(),
                                    });
                                }
                            }

                            WsClientMessage::Message(envelope) => {
                                if let (Some(ref uid), Some(ref did)) = (&user_id, &device_id) {
                                    // Verify sender
                                    if envelope.sender_id != *uid {
                                        let _ = tx.send(WsServerMessage::Error {
                                            code: "INVALID_SENDER".to_string(),
                                            message: "Sender ID mismatch".to_string(),
                                        });
                                        continue;
                                    }

                                    // Try to deliver directly if recipient is online
                                    if state.ws_manager.is_user_online(&envelope.recipient_id) {
                                        if let Some(ref device) = envelope.recipient_device_id {
                                            state.ws_manager.send_to_device(
                                                device,
                                                WsServerMessage::Message(envelope.clone()),
                                            );
                                        } else {
                                            state.ws_manager.send_to_user(
                                                &envelope.recipient_id,
                                                WsServerMessage::Message(envelope.clone()),
                                            );
                                        }
                                    }

                                    // Store for offline delivery
                                    let _ = state.storage.store_pending_message(
                                        &envelope,
                                        state.config.storage.max_message_age_hours as i64,
                                    ).await;

                                    // Acknowledge to sender
                                    let msg_id = envelope.message_id.clone();

                                    // Sync to other devices of sender
                                    state.ws_manager.send_to_other_devices(
                                        uid,
                                        did,
                                        WsServerMessage::Message(envelope),
                                    );

                                    let _ = tx.send(WsServerMessage::Acknowledged {
                                        message_ids: vec![msg_id],
                                    });
                                }
                            }

                            WsClientMessage::Acknowledge { message_ids } => {
                                let _ = state.storage.delete_pending_messages(&message_ids).await;
                                let _ = tx.send(WsServerMessage::Acknowledged { message_ids });
                            }

                            WsClientMessage::Typing { recipient_id, is_typing } => {
                                if let Some(ref uid) = user_id {
                                    state.ws_manager.send_to_user(
                                        &recipient_id,
                                        WsServerMessage::Typing {
                                            user_id: uid.clone(),
                                            is_typing,
                                        },
                                    );
                                }
                            }

                            WsClientMessage::Presence { status } => {
                                if let Some(ref uid) = user_id {
                                    // Broadcast to contacts (in production, you'd have a contacts list)
                                    let online_users = state.ws_manager.get_online_users();
                                    for other_user in online_users {
                                        if other_user != *uid {
                                            state.ws_manager.send_to_user(
                                                &other_user,
                                                WsServerMessage::Presence {
                                                    user_id: uid.clone(),
                                                    status: status.clone(),
                                                },
                                            );
                                        }
                                    }
                                }
                            }

                            WsClientMessage::CallSignal(signal) => {
                                if let Some(ref uid) = user_id {
                                    // Verify sender
                                    if signal.sender_id != *uid {
                                        continue;
                                    }

                                    // Forward call signal to recipient
                                    let recipient = signal.recipient_id.clone();
                                    state.ws_manager.send_to_user(
                                        &recipient,
                                        WsServerMessage::CallSignal(signal),
                                    );
                                }
                            }

                            WsClientMessage::Ping => {
                                let _ = tx.send(WsServerMessage::Pong);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse WebSocket message: {}", e);
                        let _ = tx.send(WsServerMessage::Error {
                            code: "PARSE_ERROR".to_string(),
                            message: format!("Invalid message format: {}", e),
                        });
                    }
                }
            }
            Ok(Message::Binary(_)) => {
                // Binary messages not supported
            }
            Ok(Message::Ping(_)) => {
                // Handled by the WebSocket library
            }
            Ok(Message::Pong(_)) => {
                // Ignore pongs
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                tracing::warn!("WebSocket error: {}", e);
                break;
            }
        }
    }

    // Cleanup
    if let Some(did) = device_id {
        state.ws_manager.unregister(&did);

        if let Some(uid) = user_id {
            // Update last seen
            let _ = state.storage.update_user_last_seen(&uid).await;

            // If no more devices online, broadcast offline status
            if !state.ws_manager.is_user_online(&uid) {
                let online_users = state.ws_manager.get_online_users();
                state.ws_manager.broadcast_user_offline(&uid, &online_users);
            }
        }
    }

    // Abort send task
    send_task.abort();
}
