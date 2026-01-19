//! Error types for PrivMsg Core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("No public key for user: {0}")]
    NoPublicKey(String),

    #[error("No session with user: {0}")]
    NoSession(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("HTTP error: {0}")]
    Http(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(e.to_string())
    }
}
