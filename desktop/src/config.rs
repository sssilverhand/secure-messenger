//! Configuration management for PrivMsg Desktop

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub ui: UiConfig,
    pub notifications: NotificationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String, // "dark" or "light"
    pub font_size: f32,
    pub compact_mode: bool,
    pub show_avatars: bool,
    pub enter_to_send: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub sound: bool,
    pub preview: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: String::new(),
                port: 8443,
                use_tls: true,
            },
            ui: UiConfig {
                theme: "dark".to_string(),
                font_size: 14.0,
                compact_mode: false,
                show_avatars: true,
                enter_to_send: true,
            },
            notifications: NotificationConfig {
                enabled: true,
                sound: true,
                preview: true,
            },
        }
    }
}

impl AppConfig {
    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        let config_path = data_dir.join("config.json");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Self = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        let config_path = data_dir.join("config.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn http_url(&self) -> String {
        let scheme = if self.server.use_tls { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.server.host, self.server.port)
    }

    pub fn ws_url(&self) -> String {
        let scheme = if self.server.use_tls { "wss" } else { "ws" };
        format!("{}://{}:{}/ws", scheme, self.server.host, self.server.port)
    }
}
