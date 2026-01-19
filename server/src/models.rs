//! Data models for PrivMsg Server

use serde::{Deserialize, Serialize};

// ============================================================================
// User Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub user_id: String,
    pub key_hash: String,
    pub display_name: Option<String>,
    pub avatar_file_id: Option<String>,
    pub public_key: Option<String>, // Base64-encoded public key for E2EE
    pub created_at: String,
    pub last_seen_at: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_file_id: Option<String>,
    pub public_key: Option<String>,
    pub last_seen_at: Option<String>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            user_id: user.user_id,
            display_name: user.display_name,
            avatar_file_id: user.avatar_file_id,
            public_key: user.public_key,
            last_seen_at: user.last_seen_at,
        }
    }
}

// ============================================================================
// Device Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Device {
    pub device_id: String,
    pub user_id: String,
    pub device_name: String,
    pub device_type: String, // "android", "windows", "linux"
    pub push_token: Option<String>,
    pub public_key: String, // Per-device public key for multi-device E2EE
    pub created_at: String,
    pub last_active_at: String,
}

// ============================================================================
// Session Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub token_hash: String,
    pub user_id: String,
    pub device_id: String,
    pub created_at: String,
    pub expires_at: String,
    pub is_valid: bool,
}

// ============================================================================
// Message Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingMessage {
    pub id: i64,
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub recipient_device_id: Option<String>, // None = all devices
    pub encrypted_content: String, // Base64-encoded encrypted message
    pub message_type: String,      // "text", "voice", "video", "file", "call_signal"
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub recipient_device_id: Option<String>,
    pub encrypted_content: String,
    pub message_type: MessageType,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Voice,
    Video,
    File,
    Image,
    CallSignal,
    KeyExchange,
    ReadReceipt,
    TypingIndicator,
    DeviceSync,
}

impl ToString for MessageType {
    fn to_string(&self) -> String {
        match self {
            MessageType::Text => "text".to_string(),
            MessageType::Voice => "voice".to_string(),
            MessageType::Video => "video".to_string(),
            MessageType::File => "file".to_string(),
            MessageType::Image => "image".to_string(),
            MessageType::CallSignal => "call_signal".to_string(),
            MessageType::KeyExchange => "key_exchange".to_string(),
            MessageType::ReadReceipt => "read_receipt".to_string(),
            MessageType::TypingIndicator => "typing_indicator".to_string(),
            MessageType::DeviceSync => "device_sync".to_string(),
        }
    }
}

impl From<String> for MessageType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "text" => MessageType::Text,
            "voice" => MessageType::Voice,
            "video" => MessageType::Video,
            "file" => MessageType::File,
            "image" => MessageType::Image,
            "call_signal" => MessageType::CallSignal,
            "key_exchange" => MessageType::KeyExchange,
            "read_receipt" => MessageType::ReadReceipt,
            "typing_indicator" => MessageType::TypingIndicator,
            "device_sync" => MessageType::DeviceSync,
            _ => MessageType::Text,
        }
    }
}

// ============================================================================
// File Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileMetadata {
    pub file_id: String,
    pub uploader_id: String,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: String,
    pub encryption_key_hash: String, // Hash of the encryption key (for verification)
    pub created_at: String,
    pub expires_at: String,
    pub download_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub file_id: String,
    pub upload_url: Option<String>,
    pub expires_at: i64,
}

// ============================================================================
// WebSocket Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsClientMessage {
    #[serde(rename = "authenticate")]
    Authenticate { token: String },

    #[serde(rename = "message")]
    Message(MessageEnvelope),

    #[serde(rename = "ack")]
    Acknowledge { message_ids: Vec<String> },

    #[serde(rename = "typing")]
    Typing { recipient_id: String, is_typing: bool },

    #[serde(rename = "presence")]
    Presence { status: PresenceStatus },

    #[serde(rename = "call_signal")]
    CallSignal(CallSignal),

    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsServerMessage {
    #[serde(rename = "authenticated")]
    Authenticated { user_id: String, device_id: String },

    #[serde(rename = "error")]
    Error { code: String, message: String },

    #[serde(rename = "message")]
    Message(MessageEnvelope),

    #[serde(rename = "ack")]
    Acknowledged { message_ids: Vec<String> },

    #[serde(rename = "typing")]
    Typing { user_id: String, is_typing: bool },

    #[serde(rename = "presence")]
    Presence { user_id: String, status: PresenceStatus },

    #[serde(rename = "call_signal")]
    CallSignal(CallSignal),

    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "user_online")]
    UserOnline { user_id: String },

    #[serde(rename = "user_offline")]
    UserOffline { user_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Away,
    Offline,
}

// ============================================================================
// Call Signaling Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSignal {
    pub call_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub signal_type: CallSignalType,
    pub payload: String, // JSON-encoded SDP or ICE candidate
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CallSignalType {
    Offer,
    Answer,
    IceCandidate,
    Hangup,
    Busy,
    Ringing,
    Accepted,
    Rejected,
}

// ============================================================================
// API Request/Response Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub user_id: String,
    pub access_key: String,
    pub device_name: String,
    pub device_type: String,
    pub device_public_key: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub device_id: String,
    pub expires_at: i64,
    pub user: UserProfile,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub expires_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub avatar_file_id: Option<String>,
    pub public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AcknowledgeMessagesRequest {
    pub message_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TurnCredentialsResponse {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
    pub credential_type: String,
    pub ttl: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub admin_key: String,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateUserResponse {
    pub user_id: String,
    pub access_key: String,
}

#[derive(Debug, Serialize)]
pub struct ServerStats {
    pub total_users: i64,
    pub active_users: i64,
    pub online_users: i64,
    pub pending_messages: i64,
    pub stored_files: i64,
    pub storage_used_mb: f64,
}
