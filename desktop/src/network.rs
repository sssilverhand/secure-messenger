//! Network layer for PrivMsg Desktop

use crate::config::AppConfig;
use crate::crypto::CryptoEngine;
use crate::state::{
    Attachment, AuthSession, ChatMessage, MessageStatus, MessageType, User,
};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

// ============================================================================
// WebSocket Event
// ============================================================================

#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected,
    Disconnected,
    Message(MessageEnvelope),
    CallSignal(CallSignal),
    Typing { user_id: String, is_typing: bool },
    Presence { user_id: String, status: String },
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSignal {
    pub call_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub signal_type: String,
    pub payload: String,
}

// ============================================================================
// Network Client
// ============================================================================

pub struct NetworkClient {
    http: Client,
    base_url: String,
    ws_url: String,
    token: Mutex<Option<String>>,
    user_id: Mutex<Option<String>>,
    crypto: Arc<CryptoEngine>,
    ws_sender: Mutex<Option<mpsc::UnboundedSender<String>>>,
    incoming_events: Arc<Mutex<VecDeque<WsEvent>>>,
}

impl NetworkClient {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let http = Client::builder()
            .danger_accept_invalid_certs(!config.server.use_tls)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let crypto = Arc::new(CryptoEngine::new());

        Ok(Self {
            http,
            base_url: config.http_url(),
            ws_url: config.ws_url(),
            token: Mutex::new(None),
            user_id: Mutex::new(None),
            crypto,
            ws_sender: Mutex::new(None),
            incoming_events: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    fn auth_header(&self) -> Option<String> {
        self.token.lock().as_ref().map(|t| format!("Bearer {}", t))
    }

    // ============= Authentication =============

    pub async fn login(
        &self,
        user_id: &str,
        access_key: &str,
        device_name: &str,
    ) -> Result<AuthSession> {
        // Generate device keys
        self.crypto.generate_identity()?;
        let public_key = self.crypto.get_public_key()?;

        let resp = self
            .http
            .post(format!("{}/api/v1/auth/login", self.base_url))
            .json(&json!({
                "user_id": user_id,
                "access_key": access_key,
                "device_name": device_name,
                "device_type": std::env::consts::OS,
                "device_public_key": public_key
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Login failed: {} - {}", status, text));
        }

        let data: serde_json::Value = resp.json().await?;

        let session = AuthSession {
            token: data["token"].as_str().unwrap_or_default().to_string(),
            device_id: data["device_id"].as_str().unwrap_or_default().to_string(),
            user_id: user_id.to_string(),
            expires_at: data["expires_at"].as_i64().unwrap_or(0),
        };

        *self.token.lock() = Some(session.token.clone());
        *self.user_id.lock() = Some(user_id.to_string());

        // Connect WebSocket
        self.connect_websocket(&session.token).await?;

        Ok(session)
    }

    pub async fn validate_token(&self, token: &str) -> Result<bool> {
        let resp = self
            .http
            .get(format!("{}/api/v1/users/me", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if resp.status().is_success() {
            *self.token.lock() = Some(token.to_string());
            self.connect_websocket(token).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn logout(&self) -> Result<()> {
        if let Some(auth) = self.auth_header() {
            self.http
                .post(format!("{}/api/v1/auth/logout", self.base_url))
                .header("Authorization", auth)
                .send()
                .await
                .ok();
        }
        *self.token.lock() = None;
        *self.user_id.lock() = None;
        *self.ws_sender.lock() = None;
        Ok(())
    }

    // ============= WebSocket =============

    async fn connect_websocket(&self, token: &str) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        *self.ws_sender.lock() = Some(tx);

        let incoming = self.incoming_events.clone();

        // Authenticate
        let auth_msg = json!({
            "type": "authenticate",
            "payload": { "token": token }
        });
        write.send(WsMessage::Text(auth_msg.to_string())).await?;

        // Receive task
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                            let event = match data["type"].as_str() {
                                Some("message") => {
                                    if let Some(payload) = data.get("payload") {
                                        serde_json::from_value::<MessageEnvelope>(payload.clone())
                                            .ok()
                                            .map(WsEvent::Message)
                                    } else {
                                        None
                                    }
                                }
                                Some("call_signal") => {
                                    if let Some(payload) = data.get("payload") {
                                        serde_json::from_value::<CallSignal>(payload.clone())
                                            .ok()
                                            .map(WsEvent::CallSignal)
                                    } else {
                                        None
                                    }
                                }
                                Some("typing") => {
                                    if let Some(payload) = data.get("payload") {
                                        Some(WsEvent::Typing {
                                            user_id: payload["user_id"]
                                                .as_str()
                                                .unwrap_or_default()
                                                .to_string(),
                                            is_typing: payload["is_typing"].as_bool().unwrap_or(false),
                                        })
                                    } else {
                                        None
                                    }
                                }
                                Some("presence") => {
                                    if let Some(payload) = data.get("payload") {
                                        Some(WsEvent::Presence {
                                            user_id: payload["user_id"]
                                                .as_str()
                                                .unwrap_or_default()
                                                .to_string(),
                                            status: payload["status"]
                                                .as_str()
                                                .unwrap_or("offline")
                                                .to_string(),
                                        })
                                    } else {
                                        None
                                    }
                                }
                                Some("authenticated") => Some(WsEvent::Connected),
                                _ => None,
                            };

                            if let Some(event) = event {
                                incoming.lock().push_back(event);
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) | Err(_) => {
                        incoming.lock().push_back(WsEvent::Disconnected);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Send task
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if write.send(WsMessage::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        Ok(())
    }

    fn send_ws(&self, msg: serde_json::Value) -> Result<()> {
        if let Some(ref sender) = *self.ws_sender.lock() {
            sender.send(msg.to_string())?;
        }
        Ok(())
    }

    pub fn poll_events(&self) -> Vec<WsEvent> {
        let mut events = Vec::new();
        let mut queue = self.incoming_events.lock();
        while let Some(event) = queue.pop_front() {
            events.push(event);
        }
        events
    }

    // ============= Users =============

    pub async fn find_user(&self, user_id: &str) -> Result<User> {
        let auth = self.auth_header().ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        let resp = self
            .http
            .get(format!("{}/api/v1/users/{}", self.base_url, user_id))
            .header("Authorization", auth)
            .send()
            .await?;

        if resp.status().as_u16() == 404 {
            return Err(anyhow::anyhow!("User not found"));
        }

        let data: serde_json::Value = resp.json().await?;

        Ok(User {
            user_id: data["user_id"].as_str().unwrap_or_default().to_string(),
            display_name: data["display_name"].as_str().map(|s| s.to_string()),
            avatar_file_id: data["avatar_file_id"].as_str().map(|s| s.to_string()),
            public_key: data["public_key"].as_str().map(|s| s.to_string()),
            last_seen_at: data["last_seen_at"].as_i64(),
        })
    }

    // ============= Messaging =============

    pub async fn send_text_message(&self, recipient_id: &str, text: &str) -> Result<ChatMessage> {
        let sender_id = self.user_id.lock().clone().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

        // Get recipient's public key if we don't have a session
        if !self.crypto.has_session(recipient_id) {
            let user = self.find_user(recipient_id).await?;
            if let Some(pub_key) = user.public_key {
                self.crypto.establish_session(recipient_id, &pub_key)?;
            } else {
                return Err(anyhow::anyhow!("Recipient has no public key"));
            }
        }

        // Encrypt message
        let content = json!({ "text": text });
        let encrypted = self.crypto.encrypt_for(recipient_id, &content.to_string())?;

        let message_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();

        let envelope = MessageEnvelope {
            message_id: message_id.clone(),
            sender_id: sender_id.clone(),
            recipient_id: recipient_id.to_string(),
            recipient_device_id: None,
            encrypted_content: encrypted,
            message_type: "text".to_string(),
            timestamp,
        };

        // Send via WebSocket
        self.send_ws(json!({
            "type": "message",
            "payload": envelope
        }))?;

        Ok(ChatMessage {
            message_id,
            conversation_id: recipient_id.to_string(),
            sender_id,
            message_type: MessageType::Text,
            content: text.to_string(),
            timestamp,
            status: MessageStatus::Sent,
            attachment: None,
            is_outgoing: true,
        })
    }

    pub async fn send_file_message(
        &self,
        recipient_id: &str,
        data: Vec<u8>,
        file_name: &str,
        mime_type: &str,
    ) -> Result<ChatMessage> {
        let sender_id = self.user_id.lock().clone().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

        // Generate file encryption key
        let file_key = self.crypto.generate_file_key()?;

        // Encrypt file
        let encrypted_data = self.crypto.encrypt_file(&data, &file_key)?;

        // Upload encrypted file
        let file_id = self.upload_file(encrypted_data, file_name, mime_type, &file_key).await?;

        // Ensure we have session with recipient
        if !self.crypto.has_session(recipient_id) {
            let user = self.find_user(recipient_id).await?;
            if let Some(pub_key) = user.public_key {
                self.crypto.establish_session(recipient_id, &pub_key)?;
            } else {
                return Err(anyhow::anyhow!("Recipient has no public key"));
            }
        }

        // Create message content with file info
        let content = json!({
            "file_id": file_id,
            "file_name": file_name,
            "file_size": data.len(),
            "mime_type": mime_type,
            "encryption_key": file_key
        });
        let encrypted = self.crypto.encrypt_for(recipient_id, &content.to_string())?;

        let message_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();

        let msg_type = if mime_type.starts_with("image/") {
            "image"
        } else if mime_type.starts_with("audio/") {
            "voice"
        } else if mime_type.starts_with("video/") {
            "video"
        } else {
            "file"
        };

        let envelope = MessageEnvelope {
            message_id: message_id.clone(),
            sender_id: sender_id.clone(),
            recipient_id: recipient_id.to_string(),
            recipient_device_id: None,
            encrypted_content: encrypted,
            message_type: msg_type.to_string(),
            timestamp,
        };

        self.send_ws(json!({
            "type": "message",
            "payload": envelope
        }))?;

        let message_type = match msg_type {
            "image" => MessageType::Image,
            "voice" => MessageType::Voice,
            "video" => MessageType::Video,
            _ => MessageType::File,
        };

        Ok(ChatMessage {
            message_id,
            conversation_id: recipient_id.to_string(),
            sender_id,
            message_type,
            content: file_name.to_string(),
            timestamp,
            status: MessageStatus::Sent,
            attachment: Some(Attachment {
                file_id,
                file_name: file_name.to_string(),
                file_size: data.len() as i64,
                mime_type: mime_type.to_string(),
                duration_ms: None,
                width: None,
                height: None,
                encryption_key: Some(file_key),
                local_path: None,
            }),
            is_outgoing: true,
        })
    }

    pub async fn send_voice_message(
        &self,
        recipient_id: &str,
        audio_data: Vec<u8>,
        duration_ms: i64,
    ) -> Result<ChatMessage> {
        let sender_id = self.user_id.lock().clone().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

        // Generate file encryption key
        let file_key = self.crypto.generate_file_key()?;

        // Encrypt audio
        let encrypted_data = self.crypto.encrypt_file(&audio_data, &file_key)?;

        // Upload
        let file_id = self.upload_file(encrypted_data, "voice.ogg", "audio/ogg", &file_key).await?;

        // Ensure session
        if !self.crypto.has_session(recipient_id) {
            let user = self.find_user(recipient_id).await?;
            if let Some(pub_key) = user.public_key {
                self.crypto.establish_session(recipient_id, &pub_key)?;
            }
        }

        let content = json!({
            "file_id": file_id,
            "file_name": "voice.ogg",
            "file_size": audio_data.len(),
            "mime_type": "audio/ogg",
            "duration_ms": duration_ms,
            "encryption_key": file_key
        });
        let encrypted = self.crypto.encrypt_for(recipient_id, &content.to_string())?;

        let message_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();

        let envelope = MessageEnvelope {
            message_id: message_id.clone(),
            sender_id: sender_id.clone(),
            recipient_id: recipient_id.to_string(),
            recipient_device_id: None,
            encrypted_content: encrypted,
            message_type: "voice".to_string(),
            timestamp,
        };

        self.send_ws(json!({
            "type": "message",
            "payload": envelope
        }))?;

        Ok(ChatMessage {
            message_id,
            conversation_id: recipient_id.to_string(),
            sender_id,
            message_type: MessageType::Voice,
            content: format!("Voice message ({}s)", duration_ms / 1000),
            timestamp,
            status: MessageStatus::Sent,
            attachment: Some(Attachment {
                file_id,
                file_name: "voice.ogg".to_string(),
                file_size: audio_data.len() as i64,
                mime_type: "audio/ogg".to_string(),
                duration_ms: Some(duration_ms),
                width: None,
                height: None,
                encryption_key: Some(file_key),
                local_path: None,
            }),
            is_outgoing: true,
        })
    }

    // ============= Files =============

    async fn upload_file(
        &self,
        data: Vec<u8>,
        file_name: &str,
        mime_type: &str,
        encryption_key: &str,
    ) -> Result<String> {
        let auth = self.auth_header().ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        let key_hash = self.crypto.hash(encryption_key.as_bytes());

        let part = reqwest::multipart::Part::bytes(data)
            .file_name(file_name.to_string())
            .mime_str(mime_type)?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("encryption_key_hash", key_hash);

        let resp = self
            .http
            .post(format!("{}/api/v1/files/upload", self.base_url))
            .header("Authorization", auth)
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Upload failed: {}", resp.status()));
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data["file_id"].as_str().unwrap_or_default().to_string())
    }

    pub async fn download_file(&self, file_id: &str) -> Result<Vec<u8>> {
        let auth = self.auth_header().ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        let resp = self
            .http
            .get(format!("{}/api/v1/files/{}", self.base_url, file_id))
            .header("Authorization", auth)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Download failed: {}", resp.status()));
        }

        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    // ============= Calls =============

    pub async fn initiate_call(&self, peer_id: &str, is_video: bool) -> Result<String> {
        let sender_id = self.user_id.lock().clone().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;
        let call_id = uuid::Uuid::new_v4().to_string();

        // In a real implementation, this would create a WebRTC offer
        let offer_payload = json!({
            "type": "offer",
            "sdp": "placeholder", // Would be actual SDP
            "video": is_video
        });

        let signal = CallSignal {
            call_id: call_id.clone(),
            sender_id,
            recipient_id: peer_id.to_string(),
            signal_type: "offer".to_string(),
            payload: offer_payload.to_string(),
        };

        self.send_ws(json!({
            "type": "call_signal",
            "payload": signal
        }))?;

        Ok(call_id)
    }

    pub async fn accept_call(&self, call_id: &str) -> Result<()> {
        let sender_id = self.user_id.lock().clone().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

        // In real implementation, this would create a WebRTC answer
        let answer_payload = json!({
            "type": "answer",
            "sdp": "placeholder" // Would be actual SDP
        });

        // Note: In real implementation, recipient_id would come from the call state
        let signal = CallSignal {
            call_id: call_id.to_string(),
            sender_id,
            recipient_id: String::new(), // Would be filled from call state
            signal_type: "answer".to_string(),
            payload: answer_payload.to_string(),
        };

        self.send_ws(json!({
            "type": "call_signal",
            "payload": signal
        }))?;

        Ok(())
    }

    pub async fn end_call(&self, call_id: &str) -> Result<()> {
        let sender_id = self.user_id.lock().clone().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

        let signal = CallSignal {
            call_id: call_id.to_string(),
            sender_id,
            recipient_id: String::new(),
            signal_type: "hangup".to_string(),
            payload: "{}".to_string(),
        };

        self.send_ws(json!({
            "type": "call_signal",
            "payload": signal
        }))?;

        Ok(())
    }

    pub async fn get_turn_credentials(&self) -> Result<TurnCredentials> {
        let auth = self.auth_header().ok_or_else(|| anyhow::anyhow!("Not authenticated"))?;

        let resp = self
            .http
            .get(format!("{}/api/v1/turn/credentials", self.base_url))
            .header("Authorization", auth)
            .send()
            .await?;

        let creds: TurnCredentials = resp.json().await?;
        Ok(creds)
    }

    // ============= Typing indicator =============

    pub fn send_typing(&self, recipient_id: &str, is_typing: bool) -> Result<()> {
        self.send_ws(json!({
            "type": "typing",
            "payload": {
                "recipient_id": recipient_id,
                "is_typing": is_typing
            }
        }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCredentials {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}
