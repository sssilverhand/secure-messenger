//! Application messages (events)

use crate::network::WsEvent;
use crate::state::{AuthSession, ChatMessage, Conversation, Screen, User};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    NavigateTo(Screen),
    GoBack,

    // Login
    ServerHostChanged(String),
    ServerPortChanged(String),
    UseTlsChanged(bool),
    UserIdChanged(String),
    AccessKeyChanged(String),
    Login,
    LoginSuccess(AuthSession),
    LoginError(String),
    TryRestoreSession,
    Logout,

    // Conversations
    LoadConversations,
    ConversationsLoaded(Vec<Conversation>),
    OpenChat(String),
    MessagesLoaded(Vec<ChatMessage>),

    // Messaging
    MessageInputChanged(String),
    SendMessage,
    MessageSent(ChatMessage),
    MessageReceived(ChatMessage),

    // Search
    SearchQueryChanged(String),
    SearchUser,
    UserFound(User),
    StartChatWithUser(String),
    ToggleSearch,

    // Voice recording
    StartRecordingVoice,
    StopRecordingVoice,
    CancelRecordingVoice,

    // File attachments
    AttachFile,
    FileSelected(PathBuf),
    DownloadFile(String, String), // file_id, file_name
    FileDownloaded(PathBuf),

    // Calls
    StartCall(String, bool), // peer_id, is_video
    CallInitiated(String),   // call_id
    IncomingCall(String, String, bool), // call_id, peer_id, is_video
    AcceptCall,
    RejectCall,
    EndCall,
    CallConnected,
    CallEnded,
    CallError(String),
    ToggleMute,
    ToggleVideo,

    // Settings
    OpenSettings,
    ThemeChanged(String),
    NotificationsChanged(bool),
    SoundChanged(bool),

    // WebSocket
    WebSocketEvent(WsEvent),

    // Misc
    Error(String),
    ClearError,
    Tick,
    Noop,
}
