//! PrivMsg Core Library
//!
//! Shared library for E2EE messaging across all platforms.
//! Provides: cryptography, networking, storage, and models.

pub mod crypto;
pub mod network;
pub mod storage;
pub mod models;
pub mod error;

#[cfg(target_os = "android")]
pub mod android;

use std::sync::Arc;
use parking_lot::RwLock;
use tokio::runtime::Runtime;

pub use crypto::*;
pub use network::*;
pub use storage::*;
pub use models::*;
pub use error::*;

/// Main client instance
pub struct PrivMsgClient {
    config: ClientConfig,
    crypto: Arc<CryptoEngine>,
    api: Arc<ApiClient>,
    ws: Arc<RwLock<Option<WebSocketClient>>>,
    storage: Arc<LocalStorage>,
    runtime: Runtime,
}

impl PrivMsgClient {
    /// Create new client instance
    pub fn new(config: ClientConfig, data_dir: &str) -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| Error::Runtime(e.to_string()))?;

        let storage = Arc::new(LocalStorage::new(data_dir)?);
        let crypto = Arc::new(CryptoEngine::new());
        let api = Arc::new(ApiClient::new(&config));

        Ok(Self {
            config,
            crypto,
            api,
            ws: Arc::new(RwLock::new(None)),
            storage,
            runtime,
        })
    }

    /// Initialize crypto keys (load existing or generate new)
    pub fn init_keys(&self, private_key: Option<&str>) -> Result<String> {
        match private_key {
            Some(key) => {
                self.crypto.import_identity(key)?;
            }
            None => {
                self.crypto.generate_identity()?;
            }
        }
        self.crypto.get_public_key()
    }

    /// Login to server
    pub fn login(&self, user_id: &str, access_key: &str, device_name: &str) -> Result<AuthSession> {
        let public_key = self.crypto.get_public_key()?;

        self.runtime.block_on(async {
            let session = self.api.login(user_id, access_key, device_name, &public_key).await?;

            // Save session
            self.storage.save_session(&session)?;

            // Connect WebSocket
            let ws = WebSocketClient::connect(&self.config, &session.token).await?;
            *self.ws.write() = Some(ws);

            Ok(session)
        })
    }

    /// Send text message
    pub fn send_message(&self, recipient_id: &str, text: &str) -> Result<Message> {
        // Ensure we have session with recipient
        if !self.crypto.has_session(recipient_id) {
            // Fetch recipient's public key
            let user = self.runtime.block_on(self.api.get_user(recipient_id))?;
            if let Some(pub_key) = user.public_key {
                self.crypto.establish_session(recipient_id, &pub_key)?;
            } else {
                return Err(Error::NoPublicKey(recipient_id.to_string()));
            }
        }

        // Encrypt message
        let content = serde_json::json!({ "text": text });
        let encrypted = self.crypto.encrypt_for(recipient_id, &content.to_string())?;

        let message_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();

        let envelope = MessageEnvelope {
            message_id: message_id.clone(),
            sender_id: self.get_current_user_id()?,
            recipient_id: recipient_id.to_string(),
            recipient_device_id: None,
            encrypted_content: encrypted,
            message_type: "text".to_string(),
            timestamp,
        };

        // Send via WebSocket
        if let Some(ref ws) = *self.ws.read() {
            self.runtime.block_on(ws.send_message(&envelope))?;
        }

        // Save locally
        let message = Message {
            message_id,
            conversation_id: recipient_id.to_string(),
            sender_id: self.get_current_user_id()?,
            message_type: MessageType::Text,
            content: text.to_string(),
            timestamp,
            status: MessageStatus::Sent,
            attachment: None,
            is_outgoing: true,
        };

        self.storage.save_message(&message)?;

        Ok(message)
    }

    /// Get conversations list
    pub fn get_conversations(&self) -> Result<Vec<Conversation>> {
        self.storage.get_conversations()
    }

    /// Get messages for conversation
    pub fn get_messages(&self, conversation_id: &str, limit: i64, offset: i64) -> Result<Vec<Message>> {
        self.storage.get_messages(conversation_id, limit, offset)
    }

    /// Get current user ID
    pub fn get_current_user_id(&self) -> Result<String> {
        self.storage.get_setting("current_user_id")
            .ok_or_else(|| Error::NotLoggedIn)
    }

    /// Export private key for backup
    pub fn export_private_key(&self) -> Result<String> {
        self.crypto.export_identity()
    }

    /// Logout
    pub fn logout(&self) -> Result<()> {
        if let Some(ref ws) = *self.ws.write().take() {
            self.runtime.block_on(ws.disconnect())?;
        }
        self.storage.clear_session()?;
        Ok(())
    }

    /// Poll for new messages (call periodically)
    pub fn poll_messages(&self) -> Result<Vec<Message>> {
        let ws_guard = self.ws.read();
        if let Some(ref ws) = *ws_guard {
            let envelopes = self.runtime.block_on(ws.receive_messages())?;
            drop(ws_guard);

            let mut messages = Vec::new();
            for envelope in envelopes {
                if let Ok(msg) = self.process_incoming_message(envelope) {
                    messages.push(msg);
                }
            }
            return Ok(messages);
        }
        Ok(vec![])
    }

    fn process_incoming_message(&self, envelope: MessageEnvelope) -> Result<Message> {
        // Establish session if needed
        if !self.crypto.has_session(&envelope.sender_id) {
            let user = self.runtime.block_on(self.api.get_user(&envelope.sender_id))?;
            if let Some(pub_key) = user.public_key {
                self.crypto.establish_session(&envelope.sender_id, &pub_key)?;
            }
        }

        // Decrypt
        let decrypted = self.crypto.decrypt_from(&envelope.sender_id, &envelope.encrypted_content)?;
        let content: serde_json::Value = serde_json::from_str(&decrypted)?;
        let text = content["text"].as_str().unwrap_or("").to_string();

        let message = Message {
            message_id: envelope.message_id,
            conversation_id: envelope.sender_id.clone(),
            sender_id: envelope.sender_id,
            message_type: MessageType::Text,
            content: text,
            timestamp: envelope.timestamp,
            status: MessageStatus::Delivered,
            attachment: None,
            is_outgoing: false,
        };

        self.storage.save_message(&message)?;

        Ok(message)
    }
}

/// Client configuration
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub server_host: String,
    pub server_port: u16,
    pub use_tls: bool,
}

impl ClientConfig {
    pub fn new(host: &str, port: u16, use_tls: bool) -> Self {
        Self {
            server_host: host.to_string(),
            server_port: port,
            use_tls,
        }
    }

    pub fn http_url(&self) -> String {
        let scheme = if self.use_tls { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.server_host, self.server_port)
    }

    pub fn ws_url(&self) -> String {
        let scheme = if self.use_tls { "wss" } else { "ws" };
        format!("{}://{}:{}/ws", scheme, self.server_host, self.server_port)
    }
}

// C FFI exports for cross-language usage
#[no_mangle]
pub extern "C" fn privmsg_version() -> *const std::ffi::c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const std::ffi::c_char
}
