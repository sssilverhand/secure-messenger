//! Data models for PrivMsg

use serde::{Deserialize, Serialize};

// ============================================================================
// User
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_file_id: Option<String>,
    pub public_key: Option<String>,
    pub last_seen_at: Option<i64>,
}

// ============================================================================
// Session
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub token: String,
    pub device_id: String,
    pub user_id: String,
    pub expires_at: i64,
}

// ============================================================================
// Messages
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Voice,
    Video,
    Image,
    File,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
}

impl Default for MessageStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub message_type: MessageType,
    pub content: String,
    pub timestamp: i64,
    pub status: MessageStatus,
    pub attachment: Option<Attachment>,
    pub is_outgoing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub file_id: String,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub duration_ms: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub encryption_key: Option<String>,
    pub local_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub recipient_device_id: Option<String>,
    pub encrypted_content: String,
    pub message_type: String,
    pub timestamp: i64,
}

// ============================================================================
// Conversation
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub peer_id: String,
    pub peer_name: Option<String>,
    pub peer_avatar: Option<String>,
    pub last_message: Option<String>,
    pub last_message_time: Option<i64>,
    pub unread_count: i32,
    pub is_muted: bool,
    pub is_pinned: bool,
}

// ============================================================================
// Calls
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallType {
    Audio,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallState {
    Idle,
    Outgoing,
    Incoming,
    Connecting,
    Connected,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallInfo {
    pub call_id: String,
    pub peer_id: String,
    pub call_type: CallType,
    pub state: CallState,
    pub is_outgoing: bool,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSignal {
    pub call_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub signal_type: String,
    pub payload: String,
}

// ============================================================================
// TURN
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCredentials {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}
