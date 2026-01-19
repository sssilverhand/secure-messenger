//! Cryptographic utilities for PrivMsg Server
//!
//! Server-side crypto is minimal - only for:
//! - Access key generation and verification
//! - Session token management
//! - File ID generation
//!
//! All E2EE happens on the client side!

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{digest, rand::{SecureRandom, SystemRandom}};
use chrono::{Utc, Duration};
use serde::{Deserialize, Serialize};

const USER_ID_LENGTH: usize = 8;
const ACCESS_KEY_LENGTH: usize = 32;
const SESSION_TOKEN_LENGTH: usize = 32;
const FILE_ID_LENGTH: usize = 16;

/// Generate a random user ID (8 characters, alphanumeric)
pub fn generate_user_id() -> String {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; USER_ID_LENGTH];
    rng.fill(&mut bytes).expect("Failed to generate random bytes");

    // Convert to alphanumeric (base62-like)
    let chars: Vec<char> = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
        .chars()
        .collect();

    bytes.iter()
        .map(|b| chars[(*b as usize) % chars.len()])
        .collect()
}

/// Generate a random access key (base64url, 32 bytes)
pub fn generate_access_key() -> String {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; ACCESS_KEY_LENGTH];
    rng.fill(&mut bytes).expect("Failed to generate random bytes");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Hash an access key for storage (SHA-256)
pub fn hash_access_key(key: &str) -> String {
    let hash = digest::digest(&digest::SHA256, key.as_bytes());
    hex::encode(hash.as_ref())
}

/// Verify an access key against a stored hash
pub fn verify_access_key(key: &str, hash: &str) -> bool {
    let computed_hash = hash_access_key(key);
    // Constant-time comparison using constant length comparison
    if computed_hash.len() != hash.len() {
        return false;
    }
    computed_hash.as_bytes()
        .iter()
        .zip(hash.as_bytes().iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// Generate a session token
pub fn generate_session_token() -> String {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; SESSION_TOKEN_LENGTH];
    rng.fill(&mut bytes).expect("Failed to generate random bytes");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Generate a file ID
pub fn generate_file_id() -> String {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; FILE_ID_LENGTH];
    rng.fill(&mut bytes).expect("Failed to generate random bytes");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Generate a device ID
pub fn generate_device_id() -> String {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes).expect("Failed to generate random bytes");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Session token with expiry (available for future use)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub token: String,
    pub user_id: String,
    pub device_id: String,
    pub expires_at: i64,
}

#[allow(dead_code)]
impl SessionToken {
    pub fn new(user_id: &str, device_id: &str, ttl_hours: i64) -> Self {
        Self {
            token: generate_session_token(),
            user_id: user_id.to_string(),
            device_id: device_id.to_string(),
            expires_at: (Utc::now() + Duration::hours(ttl_hours)).timestamp(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expires_at
    }
}

/// Generate TURN credentials with time-limited validity
pub fn generate_turn_credentials(username: &str, secret: &str, ttl_seconds: u64) -> (String, String) {
    let timestamp = Utc::now().timestamp() as u64 + ttl_seconds;
    let turn_username = format!("{}:{}", timestamp, username);

    // HMAC-SHA1 for TURN credential
    use ring::hmac;
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret.as_bytes());
    let signature = hmac::sign(&key, turn_username.as_bytes());
    let turn_credential = base64::engine::general_purpose::STANDARD.encode(signature.as_ref());

    (turn_username, turn_credential)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_generation() {
        let id = generate_user_id();
        assert_eq!(id.len(), USER_ID_LENGTH);
        assert!(id.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_access_key_verification() {
        let key = generate_access_key();
        let hash = hash_access_key(&key);

        assert!(verify_access_key(&key, &hash));
        assert!(!verify_access_key("wrong-key", &hash));
    }

    #[test]
    fn test_session_token() {
        let token = SessionToken::new("user123", "device456", 24);
        assert!(!token.is_expired());
        assert_eq!(token.user_id, "user123");
        assert_eq!(token.device_id, "device456");
    }
}
