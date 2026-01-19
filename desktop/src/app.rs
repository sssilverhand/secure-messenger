//! Main application module for PrivMsg Desktop

use crate::config::AppConfig;
use crate::database::Database;
use crate::messages::Message;
use crate::network::NetworkClient;
use crate::screens::{
    call::CallScreen, chat::ChatScreen, home::HomeScreen, login::LoginScreen,
    settings::SettingsScreen,
};
use crate::state::{AppState, Screen};
use crate::theme::Theme;

use iced::widget::{column, container, row, text};
use iced::{executor, Application, Command, Element, Length, Subscription};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct Flags {
    pub data_dir: PathBuf,
    pub config: AppConfig,
}

pub struct PrivMsg {
    state: AppState,
    db: Arc<Database>,
    network: Arc<RwLock<Option<NetworkClient>>>,
    theme: Theme,
}

impl Application for PrivMsg {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        // Initialize database
        let db = Database::new(&flags.data_dir).expect("Failed to initialize database");
        let db = Arc::new(db);

        // Check if we have saved session
        let has_session = db.get_session().is_some();
        let has_server = !flags.config.server.host.is_empty();

        let initial_screen = if has_session && has_server {
            // Try to restore session
            Screen::Home
        } else {
            Screen::Login
        };

        let theme = if flags.config.ui.theme == "dark" {
            Theme::dark()
        } else {
            Theme::light()
        };

        let state = AppState::new(flags.data_dir, flags.config, initial_screen);

        let app = Self {
            state,
            db,
            network: Arc::new(RwLock::new(None)),
            theme,
        };

        let command = if has_session && has_server {
            Command::perform(async {}, |_| Message::TryRestoreSession)
        } else {
            Command::none()
        };

        (app, command)
    }

    fn title(&self) -> String {
        match self.state.current_screen {
            Screen::Login => "PrivMsg - Login".to_string(),
            Screen::Home => {
                if let Some(unread) = self.state.total_unread() {
                    if unread > 0 {
                        return format!("PrivMsg ({})", unread);
                    }
                }
                "PrivMsg".to_string()
            }
            Screen::Chat(ref id) => {
                if let Some(conv) = self.state.conversations.iter().find(|c| c.peer_id == *id) {
                    format!("PrivMsg - {}", conv.peer_name.as_deref().unwrap_or(id))
                } else {
                    "PrivMsg - Chat".to_string()
                }
            }
            Screen::Settings => "PrivMsg - Settings".to_string(),
            Screen::Call(_) => "PrivMsg - Call".to_string(),
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            // ============= Navigation =============
            Message::NavigateTo(screen) => {
                self.state.current_screen = screen;
                Command::none()
            }

            Message::GoBack => {
                self.state.current_screen = match &self.state.current_screen {
                    Screen::Chat(_) | Screen::Settings | Screen::Call(_) => Screen::Home,
                    _ => Screen::Login,
                };
                Command::none()
            }

            // ============= Login =============
            Message::ServerHostChanged(host) => {
                self.state.config.server.host = host;
                Command::none()
            }

            Message::ServerPortChanged(port) => {
                if let Ok(p) = port.parse() {
                    self.state.config.server.port = p;
                }
                Command::none()
            }

            Message::UseTlsChanged(use_tls) => {
                self.state.config.server.use_tls = use_tls;
                Command::none()
            }

            Message::UserIdChanged(user_id) => {
                self.state.login_user_id = user_id;
                Command::none()
            }

            Message::AccessKeyChanged(key) => {
                self.state.login_access_key = key;
                Command::none()
            }

            Message::Login => {
                self.state.is_loading = true;
                self.state.error = None;

                let config = self.state.config.clone();
                let user_id = self.state.login_user_id.clone();
                let access_key = self.state.login_access_key.clone();
                let db = self.db.clone();
                let network = self.network.clone();
                let data_dir = self.state.data_dir.clone();

                Command::perform(
                    async move {
                        // Save config
                        config.save(&data_dir).ok();

                        // Create network client
                        let client = NetworkClient::new(&config).await?;

                        // Login
                        let session = client.login(&user_id, &access_key, "Desktop").await?;

                        // Save session
                        db.save_session(&session)?;

                        // Store client
                        *network.write().await = Some(client);

                        Ok::<_, anyhow::Error>(session)
                    },
                    |result| match result {
                        Ok(session) => Message::LoginSuccess(session),
                        Err(e) => Message::LoginError(e.to_string()),
                    },
                )
            }

            Message::LoginSuccess(session) => {
                self.state.is_loading = false;
                self.state.session = Some(session);
                self.state.current_screen = Screen::Home;
                self.state.login_access_key.clear();

                // Load conversations
                Command::perform(async {}, |_| Message::LoadConversations)
            }

            Message::LoginError(error) => {
                self.state.is_loading = false;
                self.state.error = Some(error);
                Command::none()
            }

            Message::TryRestoreSession => {
                if let Some(session) = self.db.get_session() {
                    let config = self.state.config.clone();
                    let network = self.network.clone();

                    return Command::perform(
                        async move {
                            let client = NetworkClient::new(&config).await?;
                            if client.validate_token(&session.token).await? {
                                *network.write().await = Some(client);
                                Ok(session)
                            } else {
                                Err(anyhow::anyhow!("Session expired"))
                            }
                        },
                        |result| match result {
                            Ok(session) => Message::LoginSuccess(session),
                            Err(_) => Message::NavigateTo(Screen::Login),
                        },
                    );
                }
                Command::none()
            }

            // ============= Conversations =============
            Message::LoadConversations => {
                let db = self.db.clone();
                Command::perform(
                    async move { db.get_conversations() },
                    |result| match result {
                        Ok(convs) => Message::ConversationsLoaded(convs),
                        Err(e) => Message::Error(e.to_string()),
                    },
                )
            }

            Message::ConversationsLoaded(conversations) => {
                self.state.conversations = conversations;
                Command::none()
            }

            Message::OpenChat(peer_id) => {
                self.state.current_screen = Screen::Chat(peer_id.clone());
                self.state.current_chat_peer = Some(peer_id.clone());

                let db = self.db.clone();
                Command::perform(
                    async move { db.get_messages(&peer_id, 50, 0) },
                    |result| match result {
                        Ok(msgs) => Message::MessagesLoaded(msgs),
                        Err(e) => Message::Error(e.to_string()),
                    },
                )
            }

            Message::MessagesLoaded(messages) => {
                self.state.current_messages = messages;
                Command::none()
            }

            // ============= Messaging =============
            Message::MessageInputChanged(text) => {
                self.state.message_input = text;
                Command::none()
            }

            Message::SendMessage => {
                if self.state.message_input.trim().is_empty() {
                    return Command::none();
                }

                let text = self.state.message_input.clone();
                self.state.message_input.clear();

                if let Some(ref peer_id) = self.state.current_chat_peer {
                    let peer_id = peer_id.clone();
                    let network = self.network.clone();
                    let db = self.db.clone();
                    let session = self.state.session.clone();

                    return Command::perform(
                        async move {
                            if session.is_some() {
                                if let Some(ref client) = *network.read().await {
                                    let msg = client.send_text_message(&peer_id, &text).await?;
                                    db.save_message(&msg)?;
                                    return Ok(msg);
                                }
                            }
                            Err(anyhow::anyhow!("Not connected"))
                        },
                        |result| match result {
                            Ok(msg) => Message::MessageSent(msg),
                            Err(e) => Message::Error(e.to_string()),
                        },
                    );
                }
                Command::none()
            }

            Message::MessageSent(msg) => {
                self.state.current_messages.push(msg);
                Command::none()
            }

            Message::MessageReceived(msg) => {
                // Check if this message belongs to current chat
                if let Some(ref peer_id) = self.state.current_chat_peer {
                    if msg.conversation_id == *peer_id {
                        self.state.current_messages.push(msg.clone());
                    }
                }

                // Update conversation
                if let Some(conv) = self
                    .state
                    .conversations
                    .iter_mut()
                    .find(|c| c.peer_id == msg.conversation_id)
                {
                    conv.last_message = Some(msg.content.clone());
                    conv.last_message_time = Some(msg.timestamp);
                    if !msg.is_outgoing {
                        conv.unread_count += 1;
                    }
                }

                // Show notification
                if self.state.config.notifications.enabled && !msg.is_outgoing {
                    self.show_notification(&msg);
                }

                Command::none()
            }

            // ============= Search =============
            Message::SearchQueryChanged(query) => {
                self.state.search_query = query;
                Command::none()
            }

            Message::SearchUser => {
                let query = self.state.search_query.clone();
                let network = self.network.clone();

                Command::perform(
                    async move {
                        if let Some(ref client) = *network.read().await {
                            client.find_user(&query).await
                        } else {
                            Err(anyhow::anyhow!("Not connected"))
                        }
                    },
                    |result| match result {
                        Ok(user) => Message::UserFound(user),
                        Err(e) => Message::Error(e.to_string()),
                    },
                )
            }

            Message::UserFound(user) => {
                self.state.found_user = Some(user);
                Command::none()
            }

            Message::StartChatWithUser(user_id) => {
                // Create or find conversation
                if !self.state.conversations.iter().any(|c| c.peer_id == user_id) {
                    let conv = crate::state::Conversation {
                        id: user_id.clone(),
                        peer_id: user_id.clone(),
                        peer_name: self.state.found_user.as_ref().and_then(|u| u.display_name.clone()),
                        peer_avatar: None,
                        last_message: None,
                        last_message_time: None,
                        unread_count: 0,
                        is_muted: false,
                        is_pinned: false,
                    };
                    self.state.conversations.push(conv);
                    self.db.save_conversation(&self.state.conversations.last().unwrap()).ok();
                }

                self.state.found_user = None;
                self.state.search_query.clear();
                self.state.show_search = false;

                self.update(Message::OpenChat(user_id))
            }

            // ============= Calls =============
            Message::StartCall(peer_id, is_video) => {
                self.state.current_screen = Screen::Call(peer_id.clone());
                self.state.call_state = Some(crate::state::CallState::Outgoing);
                self.state.call_peer_id = Some(peer_id.clone());
                self.state.call_is_video = is_video;

                let network = self.network.clone();
                Command::perform(
                    async move {
                        if let Some(ref client) = *network.read().await {
                            client.initiate_call(&peer_id, is_video).await
                        } else {
                            Err(anyhow::anyhow!("Not connected"))
                        }
                    },
                    |result| match result {
                        Ok(call_id) => Message::CallInitiated(call_id),
                        Err(e) => Message::CallError(e.to_string()),
                    },
                )
            }

            Message::CallInitiated(call_id) => {
                self.state.call_id = Some(call_id);
                Command::none()
            }

            Message::IncomingCall(call_id, peer_id, is_video) => {
                self.state.call_id = Some(call_id);
                self.state.call_peer_id = Some(peer_id.clone());
                self.state.call_is_video = is_video;
                self.state.call_state = Some(crate::state::CallState::Incoming);
                self.state.current_screen = Screen::Call(peer_id);

                // Show notification
                if self.state.config.notifications.enabled {
                    notify_rust::Notification::new()
                        .summary("Incoming Call")
                        .body(&format!("Call from {}", self.state.call_peer_id.as_deref().unwrap_or("Unknown")))
                        .show()
                        .ok();
                }

                Command::none()
            }

            Message::AcceptCall => {
                self.state.call_state = Some(crate::state::CallState::Connecting);
                let call_id = self.state.call_id.clone();
                let network = self.network.clone();

                Command::perform(
                    async move {
                        if let (Some(call_id), Some(ref client)) = (call_id, &*network.read().await)
                        {
                            client.accept_call(&call_id).await
                        } else {
                            Err(anyhow::anyhow!("Invalid call state"))
                        }
                    },
                    |result| match result {
                        Ok(_) => Message::CallConnected,
                        Err(e) => Message::CallError(e.to_string()),
                    },
                )
            }

            Message::RejectCall | Message::EndCall => {
                let call_id = self.state.call_id.clone();
                let network = self.network.clone();

                self.state.call_state = None;
                self.state.call_id = None;
                self.state.call_peer_id = None;
                self.state.current_screen = Screen::Home;

                Command::perform(
                    async move {
                        if let (Some(call_id), Some(ref client)) = (call_id, &*network.read().await)
                        {
                            client.end_call(&call_id).await.ok();
                        }
                    },
                    |_| Message::LoadConversations,
                )
            }

            Message::CallConnected => {
                self.state.call_state = Some(crate::state::CallState::Connected);
                self.state.call_start_time = Some(chrono::Utc::now().timestamp());
                Command::none()
            }

            Message::CallEnded => {
                self.state.call_state = None;
                self.state.call_id = None;
                self.state.call_start_time = None;
                self.state.current_screen = Screen::Home;
                Command::none()
            }

            Message::CallError(error) => {
                self.state.error = Some(error);
                self.state.call_state = None;
                self.state.current_screen = Screen::Home;
                Command::none()
            }

            Message::ToggleMute => {
                self.state.call_muted = !self.state.call_muted;
                Command::none()
            }

            Message::ToggleVideo => {
                self.state.call_video_enabled = !self.state.call_video_enabled;
                Command::none()
            }

            // ============= Voice Messages =============
            Message::StartRecordingVoice => {
                self.state.is_recording_voice = true;
                self.state.recording_start_time = Some(chrono::Utc::now().timestamp());
                // TODO: Start actual recording
                Command::none()
            }

            Message::StopRecordingVoice => {
                self.state.is_recording_voice = false;
                let duration = self.state.recording_start_time.map(|start| {
                    chrono::Utc::now().timestamp() - start
                });
                self.state.recording_start_time = None;

                // TODO: Get recorded audio data and send
                if let Some(_duration) = duration {
                    // Send voice message
                }
                Command::none()
            }

            Message::CancelRecordingVoice => {
                self.state.is_recording_voice = false;
                self.state.recording_start_time = None;
                Command::none()
            }

            // ============= File Attachments =============
            Message::AttachFile => {
                Command::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Select file to send")
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |path| match path {
                        Some(p) => Message::FileSelected(p),
                        None => Message::Noop,
                    },
                )
            }

            Message::FileSelected(path) => {
                self.state.selected_file = Some(path.clone());

                if let Some(ref peer_id) = self.state.current_chat_peer {
                    let peer_id = peer_id.clone();
                    let network = self.network.clone();
                    let db = self.db.clone();

                    return Command::perform(
                        async move {
                            let data = tokio::fs::read(&path).await?;
                            let file_name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("file")
                                .to_string();
                            let mime = mime_guess::from_path(&path)
                                .first_or_octet_stream()
                                .to_string();

                            if let Some(ref client) = *network.read().await {
                                let msg = client
                                    .send_file_message(&peer_id, data, &file_name, &mime)
                                    .await?;
                                db.save_message(&msg)?;
                                return Ok(msg);
                            }
                            Err(anyhow::anyhow!("Not connected"))
                        },
                        |result| match result {
                            Ok(msg) => Message::MessageSent(msg),
                            Err(e) => Message::Error(e.to_string()),
                        },
                    );
                }
                Command::none()
            }

            Message::DownloadFile(file_id, file_name) => {
                let network = self.network.clone();

                Command::perform(
                    async move {
                        // Ask where to save
                        let path = rfd::AsyncFileDialog::new()
                            .set_title("Save file as")
                            .set_file_name(&file_name)
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf());

                        if let Some(path) = path {
                            if let Some(ref client) = *network.read().await {
                                let data = client.download_file(&file_id).await?;
                                tokio::fs::write(&path, data).await?;
                                return Ok(path);
                            }
                        }
                        Err(anyhow::anyhow!("Download cancelled"))
                    },
                    |result| match result {
                        Ok(path) => Message::FileDownloaded(path),
                        Err(e) => Message::Error(e.to_string()),
                    },
                )
            }

            Message::FileDownloaded(path) => {
                tracing::info!("File downloaded to: {:?}", path);
                Command::none()
            }

            // ============= Settings =============
            Message::OpenSettings => {
                self.state.current_screen = Screen::Settings;
                Command::none()
            }

            Message::ThemeChanged(theme) => {
                self.state.config.ui.theme = theme.clone();
                self.theme = if theme == "dark" {
                    Theme::dark()
                } else {
                    Theme::light()
                };
                self.state.config.save(&self.state.data_dir).ok();
                Command::none()
            }

            Message::NotificationsChanged(enabled) => {
                self.state.config.notifications.enabled = enabled;
                self.state.config.save(&self.state.data_dir).ok();
                Command::none()
            }

            Message::SoundChanged(enabled) => {
                self.state.config.notifications.sound = enabled;
                self.state.config.save(&self.state.data_dir).ok();
                Command::none()
            }

            Message::Logout => {
                self.db.clear_session().ok();
                self.state.session = None;
                self.state.conversations.clear();
                self.state.current_messages.clear();
                self.state.current_screen = Screen::Login;

                let network = self.network.clone();
                Command::perform(
                    async move {
                        if let Some(ref client) = *network.read().await {
                            client.logout().await.ok();
                        }
                        *network.write().await = None;
                    },
                    |_| Message::Noop,
                )
            }

            // ============= UI Toggles =============
            Message::ToggleSearch => {
                self.state.show_search = !self.state.show_search;
                if !self.state.show_search {
                    self.state.search_query.clear();
                    self.state.found_user = None;
                }
                Command::none()
            }

            // ============= Misc =============
            Message::Error(error) => {
                self.state.error = Some(error);
                self.state.is_loading = false;
                Command::none()
            }

            Message::ClearError => {
                self.state.error = None;
                Command::none()
            }

            Message::Tick => {
                // Update call duration
                if self.state.call_state == Some(crate::state::CallState::Connected) {
                    if let Some(start) = self.state.call_start_time {
                        self.state.call_duration = Some(chrono::Utc::now().timestamp() - start);
                    }
                }
                Command::none()
            }

            Message::Noop => Command::none(),

            Message::WebSocketEvent(event) => {
                // Handle WebSocket events
                match event {
                    crate::network::WsEvent::Connected => {
                        tracing::info!("WebSocket connected");
                    }
                    crate::network::WsEvent::Disconnected => {
                        tracing::warn!("WebSocket disconnected");
                        self.state.error = Some("Connection lost. Reconnecting...".to_string());
                    }
                    crate::network::WsEvent::Message(envelope) => {
                        // Decrypt and process message
                        // This is simplified - actual implementation would decrypt
                        let msg = crate::state::ChatMessage {
                            message_id: envelope.message_id,
                            conversation_id: envelope.sender_id.clone(),
                            sender_id: envelope.sender_id,
                            message_type: crate::state::MessageType::Text,
                            content: "Encrypted message".to_string(), // Would be decrypted
                            timestamp: envelope.timestamp,
                            status: crate::state::MessageStatus::Delivered,
                            attachment: None,
                            is_outgoing: false,
                        };
                        return self.update(Message::MessageReceived(msg));
                    }
                    crate::network::WsEvent::CallSignal(signal) => {
                        // Handle call signaling
                        match signal.signal_type.as_str() {
                            "offer" => {
                                return self.update(Message::IncomingCall(
                                    signal.call_id,
                                    signal.sender_id,
                                    signal.payload.contains("video"),
                                ));
                            }
                            "hangup" => {
                                return self.update(Message::CallEnded);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let content: Element<Self::Message> = match &self.state.current_screen {
            Screen::Login => LoginScreen::view(&self.state).into(),
            Screen::Home => HomeScreen::view(&self.state).into(),
            Screen::Chat(peer_id) => ChatScreen::view(&self.state, peer_id).into(),
            Screen::Settings => SettingsScreen::view(&self.state).into(),
            Screen::Call(peer_id) => CallScreen::view(&self.state, peer_id).into(),
        };

        // Wrap with error display if any
        let content = if let Some(ref error) = self.state.error {
            column![
                container(
                    row![
                        text(error).style(iced::theme::Text::Color(iced::Color::from_rgb(0.9, 0.3, 0.3))),
                        iced::widget::button(text("X"))
                            .on_press(Message::ClearError)
                            .style(iced::theme::Button::Text)
                    ]
                    .spacing(10)
                )
                .padding(10)
                .style(iced::theme::Container::Custom(Box::new(ErrorContainer))),
                content
            ]
            .into()
        } else {
            content
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let subscriptions = vec![
            // Tick every second for call duration
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick),
        ];

        // WebSocket subscription would go here
        // In a real implementation, this would subscribe to WebSocket events

        Subscription::batch(subscriptions)
    }

    fn theme(&self) -> iced::Theme {
        if self.state.config.ui.theme == "dark" {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        }
    }
}

impl PrivMsg {
    fn show_notification(&self, msg: &crate::state::ChatMessage) {
        let sender = msg.sender_id.clone();
        let body = if self.state.config.notifications.preview {
            msg.content.clone()
        } else {
            "New message".to_string()
        };

        notify_rust::Notification::new()
            .summary(&format!("Message from {}", sender))
            .body(&body)
            .show()
            .ok();
    }
}

struct ErrorContainer;

impl iced::widget::container::StyleSheet for ErrorContainer {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.3, 0.1, 0.1))),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
