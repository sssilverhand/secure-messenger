//! Network layer for PrivMsg - HTTP API and WebSocket client

use crate::error::{Error, Result};
use crate::models::*;
use crate::ClientConfig;
use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;
use reqwest::Client;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

// ============================================================================
// HTTP API Client
// ============================================================================

pub struct ApiClient {
    client: Client,
    base_url: String,
    token: Mutex<Option<String>>,
}

impl ApiClient {
    pub fn new(config: &ClientConfig) -> Self {
        let client = Client::builder()
            .danger_accept_invalid_certs(!config.use_tls) // For development
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: config.http_url(),
            token: Mutex::new(None),
        }
    }

    fn auth_header(&self) -> Option<String> {
        self.token
            .lock()
            .as_ref()
            .map(|t| format!("Bearer {}", t))
    }

    pub async fn login(
        &self,
        user_id: &str,
        access_key: &str,
        device_name: &str,
        device_public_key: &str,
    ) -> Result<AuthSession> {
        let resp = self
            .client
            .post(format!("{}/api/v1/auth/login", self.base_url))
            .json(&json!({
                "user_id": user_id,
                "access_key": access_key,
                "device_name": device_name,
                "device_type": std::env::consts::OS,
                "device_public_key": device_public_key
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Error::InvalidCredentials);
        }

        let data: serde_json::Value = resp.json().await?;

        let session = AuthSession {
            token: data["token"].as_str().unwrap_or_default().to_string(),
            device_id: data["device_id"].as_str().unwrap_or_default().to_string(),
            user_id: user_id.to_string(),
            expires_at: data["expires_at"].as_i64().unwrap_or(0),
        };

        *self.token.lock() = Some(session.token.clone());

        Ok(session)
    }

    pub async fn get_user(&self, user_id: &str) -> Result<User> {
        let mut req = self
            .client
            .get(format!("{}/api/v1/users/{}", self.base_url, user_id));

        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await?;

        if resp.status().as_u16() == 404 {
            return Err(Error::UserNotFound(user_id.to_string()));
        }

        let user: User = resp.json().await?;
        Ok(user)
    }

    pub async fn upload_file(
        &self,
        data: Vec<u8>,
        file_name: &str,
        mime_type: &str,
        encryption_key_hash: &str,
    ) -> Result<String> {
        let part = reqwest::multipart::Part::bytes(data)
            .file_name(file_name.to_string())
            .mime_str(mime_type)
            .map_err(|e| Error::Network(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("encryption_key_hash", encryption_key_hash.to_string());

        let mut req = self
            .client
            .post(format!("{}/api/v1/files/upload", self.base_url))
            .multipart(form);

        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await?;
        let data: serde_json::Value = resp.json().await?;

        Ok(data["file_id"].as_str().unwrap_or_default().to_string())
    }

    pub async fn download_file(&self, file_id: &str) -> Result<Vec<u8>> {
        let mut req = self
            .client
            .get(format!("{}/api/v1/files/{}", self.base_url, file_id));

        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await?;
        let bytes = resp.bytes().await?;

        Ok(bytes.to_vec())
    }

    pub async fn get_turn_credentials(&self) -> Result<TurnCredentials> {
        let mut req = self
            .client
            .get(format!("{}/api/v1/turn/credentials", self.base_url));

        if let Some(auth) = self.auth_header() {
            req = req.header("Authorization", auth);
        }

        let resp = req.send().await?;
        let creds: TurnCredentials = resp.json().await?;

        Ok(creds)
    }

    pub async fn check_health(&self) -> Result<bool> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await;

        match resp {
            Ok(r) => Ok(r.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

// ============================================================================
// WebSocket Client
// ============================================================================

pub struct WebSocketClient {
    sender: mpsc::UnboundedSender<String>,
    incoming: Arc<Mutex<VecDeque<MessageEnvelope>>>,
    connected: Arc<Mutex<bool>>,
}

impl WebSocketClient {
    pub async fn connect(config: &ClientConfig, token: &str) -> Result<Self> {
        let url = format!("{}", config.ws_url());
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let incoming = Arc::new(Mutex::new(VecDeque::new()));
        let connected = Arc::new(Mutex::new(true));

        let incoming_clone = incoming.clone();
        let connected_clone = connected.clone();

        // Send authentication
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
                            if data["type"] == "message" {
                                if let Some(payload) = data.get("payload") {
                                    if let Ok(envelope) =
                                        serde_json::from_value::<MessageEnvelope>(payload.clone())
                                    {
                                        incoming_clone.lock().push_back(envelope);
                                    }
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        *connected_clone.lock() = false;
                        break;
                    }
                    Err(_) => {
                        *connected_clone.lock() = false;
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

        Ok(Self {
            sender: tx,
            incoming,
            connected,
        })
    }

    pub async fn send_message(&self, envelope: &MessageEnvelope) -> Result<()> {
        let msg = json!({
            "type": "message",
            "payload": envelope
        });

        self.sender
            .send(msg.to_string())
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        Ok(())
    }

    pub async fn send_typing(&self, recipient_id: &str, is_typing: bool) -> Result<()> {
        let msg = json!({
            "type": "typing",
            "payload": {
                "recipient_id": recipient_id,
                "is_typing": is_typing
            }
        });

        self.sender
            .send(msg.to_string())
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        Ok(())
    }

    pub async fn send_call_signal(&self, signal: &CallSignal) -> Result<()> {
        let msg = json!({
            "type": "call_signal",
            "payload": signal
        });

        self.sender
            .send(msg.to_string())
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        Ok(())
    }

    pub async fn receive_messages(&self) -> Result<Vec<MessageEnvelope>> {
        let mut messages = Vec::new();
        let mut incoming = self.incoming.lock();

        while let Some(msg) = incoming.pop_front() {
            messages.push(msg);
        }

        Ok(messages)
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.lock()
    }

    pub async fn disconnect(&self) -> Result<()> {
        *self.connected.lock() = false;
        Ok(())
    }
}
