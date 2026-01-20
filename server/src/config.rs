//! Configuration management for PrivMsg Server

use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub tls: Option<TlsConfig>,
    pub turn: TurnConfig,
    pub admin: AdminConfig,
    pub limits: LimitsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub database_path: String,
    pub files_path: String,
    pub max_message_age_hours: u64,
    pub max_file_age_hours: u64,
    pub cleanup_interval_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnConfig {
    pub enabled: bool,
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
    pub credential_type: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    pub master_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_file_size_mb: u64,
    pub max_message_size_kb: u64,
    pub max_pending_messages: u64,
    pub rate_limit_messages_per_minute: u64,
}

impl Config {
    pub async fn load(path: &str) -> anyhow::Result<Self> {
        if Path::new(path).exists() {
            let content = fs::read_to_string(path).await?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            let content = toml::to_string_pretty(&config)?;
            fs::write(path, content).await?;
            tracing::info!("Created default config at {}", path);
            Ok(config)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 9443,
            },
            storage: StorageConfig {
                database_path: "./data/privmsg.db".to_string(),
                files_path: "./data/files".to_string(),
                max_message_age_hours: 168, // 7 days
                max_file_age_hours: 72,     // 3 days
                cleanup_interval_minutes: 60,
            },
            tls: None,
            turn: TurnConfig {
                enabled: true,
                urls: vec![
                    "turn:turn.example.com:3478".to_string(),
                    "turns:turn.example.com:5349".to_string(),
                ],
                username: "privmsg".to_string(),
                credential: "change-this-secret".to_string(),
                credential_type: "password".to_string(),
                ttl_seconds: 86400, // 24 hours
            },
            admin: AdminConfig {
                master_key: "CHANGE-THIS-ADMIN-KEY-IMMEDIATELY".to_string(),
            },
            limits: LimitsConfig {
                max_file_size_mb: 100,
                max_message_size_kb: 64,
                max_pending_messages: 10000,
                rate_limit_messages_per_minute: 120,
            },
        }
    }
}
