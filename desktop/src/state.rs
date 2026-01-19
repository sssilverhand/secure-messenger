//! Application state management

use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Login,
    Home,
    Chat(String), // peer_id
    Settings,
    Call(String), // peer_id
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallState {
    Idle,
    Outgoing,
    Incoming,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Voice,
    Video,
    Image,
    File,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub token: String,
    pub device_id: String,
    pub user_id: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_file_id: Option<String>,
    pub public_key: Option<String>,
    pub last_seen_at: Option<i64>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
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

pub struct AppState {
    // Paths
    pub data_dir: PathBuf,
    pub config: AppConfig,

    // Navigation
    pub current_screen: Screen,

    // Auth
    pub session: Option<AuthSession>,
    pub login_user_id: String,
    pub login_access_key: String,

    // Data
    pub conversations: Vec<Conversation>,
    pub current_messages: Vec<ChatMessage>,
    pub current_chat_peer: Option<String>,

    // Search
    pub show_search: bool,
    pub search_query: String,
    pub found_user: Option<User>,

    // Messaging
    pub message_input: String,
    pub is_recording_voice: bool,
    pub recording_start_time: Option<i64>,
    pub selected_file: Option<PathBuf>,

    // Calls
    pub call_state: Option<CallState>,
    pub call_id: Option<String>,
    pub call_peer_id: Option<String>,
    pub call_is_video: bool,
    pub call_muted: bool,
    pub call_video_enabled: bool,
    pub call_start_time: Option<i64>,
    pub call_duration: Option<i64>,

    // UI State
    pub is_loading: bool,
    pub error: Option<String>,
}

impl AppState {
    pub fn new(data_dir: PathBuf, config: AppConfig, initial_screen: Screen) -> Self {
        Self {
            data_dir,
            config,
            current_screen: initial_screen,
            session: None,
            login_user_id: String::new(),
            login_access_key: String::new(),
            conversations: Vec::new(),
            current_messages: Vec::new(),
            current_chat_peer: None,
            show_search: false,
            search_query: String::new(),
            found_user: None,
            message_input: String::new(),
            is_recording_voice: false,
            recording_start_time: None,
            selected_file: None,
            call_state: None,
            call_id: None,
            call_peer_id: None,
            call_is_video: false,
            call_muted: false,
            call_video_enabled: true,
            call_start_time: None,
            call_duration: None,
            is_loading: false,
            error: None,
        }
    }

    pub fn total_unread(&self) -> Option<i32> {
        let total: i32 = self.conversations.iter().map(|c| c.unread_count).sum();
        if total > 0 {
            Some(total)
        } else {
            None
        }
    }

    pub fn format_duration(seconds: i64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, secs)
        } else {
            format!("{:02}:{:02}", minutes, secs)
        }
    }

    pub fn format_timestamp(timestamp: i64) -> String {
        use chrono::{Datelike, Local, TimeZone, Utc};

        let dt = Utc.timestamp_opt(timestamp / 1000, 0).single();
        if let Some(dt) = dt {
            let local = dt.with_timezone(&Local);
            let now = Local::now();

            if local.date_naive() == now.date_naive() {
                // Today - show time only
                local.format("%H:%M").to_string()
            } else if local.date_naive() == (now - chrono::Duration::days(1)).date_naive() {
                // Yesterday
                "Yesterday".to_string()
            } else if local.year() == now.year() {
                // This year - show date without year
                local.format("%d %b").to_string()
            } else {
                // Other year - full date
                local.format("%d.%m.%Y").to_string()
            }
        } else {
            "".to_string()
        }
    }

    pub fn format_file_size(bytes: i64) -> String {
        const KB: i64 = 1024;
        const MB: i64 = KB * 1024;
        const GB: i64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}
