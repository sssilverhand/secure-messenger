//! Message handlers

use axum::{extract::State, Json};
use chrono::DateTime;
use crate::{
    error::Result,
    models::*,
    AppState,
};

use super::AuthUser;

/// Parse datetime string to timestamp
fn parse_datetime_to_timestamp(s: &str) -> i64 {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp())
}

/// Get pending messages for the authenticated user
pub async fn get_pending_messages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<MessageEnvelope>>> {
    let pending = state
        .storage
        .get_pending_messages(&auth.user_id, Some(&auth.device_id))
        .await?;

    let messages: Vec<MessageEnvelope> = pending
        .into_iter()
        .map(|pm| MessageEnvelope {
            message_id: pm.message_id,
            sender_id: pm.sender_id,
            recipient_id: pm.recipient_id,
            recipient_device_id: pm.recipient_device_id,
            encrypted_content: pm.encrypted_content,
            message_type: pm.message_type.into(),
            timestamp: parse_datetime_to_timestamp(&pm.created_at),
        })
        .collect();

    Ok(Json(messages))
}

/// Acknowledge (delete) received messages
pub async fn acknowledge_messages(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<AcknowledgeMessagesRequest>,
) -> Result<Json<serde_json::Value>> {
    state
        .storage
        .delete_pending_messages(&req.message_ids)
        .await?;

    Ok(Json(serde_json::json!({
        "acknowledged": req.message_ids.len()
    })))
}
