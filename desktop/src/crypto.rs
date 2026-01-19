//! E2EE Cryptography for PrivMsg Desktop

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use parking_lot::RwLock;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use x25519_dalek::{PublicKey, StaticSecret};

/// Crypto engine for E2EE operations
pub struct CryptoEngine {
    identity_secret: RwLock<Option<StaticSecret>>,
    identity_public: RwLock<Option<PublicKey>>,
    sessions: RwLock<HashMap<String, SessionKeys>>,
}

struct SessionKeys {
    shared_secret: [u8; 32],
    #[allow(dead_code)]
    created_at: i64,
}

impl CryptoEngine {
    pub fn new() -> Self {
        Self {
            identity_secret: RwLock::new(None),
            identity_public: RwLock::new(None),
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Generate new identity key pair
    pub fn generate_identity(&self) -> Result<()> {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);

        *self.identity_secret.write() = Some(secret);
        *self.identity_public.write() = Some(public);

        Ok(())
    }

    /// Import identity from base64 private key
    pub fn import_identity(&self, private_key_b64: &str) -> Result<()> {
        let bytes = URL_SAFE_NO_PAD.decode(private_key_b64)?;

        if bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid key length"));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);

        let secret = StaticSecret::from(key_bytes);
        let public = PublicKey::from(&secret);

        *self.identity_secret.write() = Some(secret);
        *self.identity_public.write() = Some(public);

        Ok(())
    }

    /// Export identity private key as base64
    pub fn export_identity(&self) -> Result<String> {
        let guard = self.identity_secret.read();
        let secret = guard.as_ref().ok_or_else(|| anyhow::anyhow!("No identity"))?;
        Ok(URL_SAFE_NO_PAD.encode(secret.as_bytes()))
    }

    /// Get public key as base64
    pub fn get_public_key(&self) -> Result<String> {
        let guard = self.identity_public.read();
        let public = guard.as_ref().ok_or_else(|| anyhow::anyhow!("No identity"))?;
        Ok(URL_SAFE_NO_PAD.encode(public.as_bytes()))
    }

    /// Establish session with another user
    pub fn establish_session(&self, peer_id: &str, peer_public_key_b64: &str) -> Result<()> {
        let peer_bytes = URL_SAFE_NO_PAD.decode(peer_public_key_b64)?;

        if peer_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid peer key length"));
        }

        let mut peer_key_bytes = [0u8; 32];
        peer_key_bytes.copy_from_slice(&peer_bytes);
        let peer_public = PublicKey::from(peer_key_bytes);

        let secret_guard = self.identity_secret.read();
        let our_secret = secret_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No identity"))?;

        let shared = our_secret.diffie_hellman(&peer_public);

        // Derive 256-bit key using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(shared.as_bytes());
        let derived = hasher.finalize();

        let mut shared_secret = [0u8; 32];
        shared_secret.copy_from_slice(&derived);

        let session = SessionKeys {
            shared_secret,
            created_at: chrono::Utc::now().timestamp(),
        };

        self.sessions.write().insert(peer_id.to_string(), session);

        Ok(())
    }

    /// Check if we have a session with a peer
    pub fn has_session(&self, peer_id: &str) -> bool {
        self.sessions.read().contains_key(peer_id)
    }

    /// Encrypt message for peer
    pub fn encrypt_for(&self, peer_id: &str, plaintext: &str) -> Result<String> {
        let sessions = self.sessions.read();
        let session = sessions
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("No session with {}", peer_id))?;

        let cipher = Aes256Gcm::new_from_slice(&session.shared_secret)?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        // Combine: nonce (12) + ciphertext + tag (16)
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(URL_SAFE_NO_PAD.encode(&combined))
    }

    /// Decrypt message from peer
    pub fn decrypt_from(&self, peer_id: &str, ciphertext_b64: &str) -> Result<String> {
        let sessions = self.sessions.read();
        let session = sessions
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("No session with {}", peer_id))?;

        let combined = URL_SAFE_NO_PAD.decode(ciphertext_b64)?;

        if combined.len() < 12 {
            return Err(anyhow::anyhow!("Ciphertext too short"));
        }

        let nonce = Nonce::from_slice(&combined[..12]);
        let ciphertext = &combined[12..];

        let cipher = Aes256Gcm::new_from_slice(&session.shared_secret)?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
    }

    /// Generate random file encryption key
    pub fn generate_file_key(&self) -> Result<String> {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Ok(URL_SAFE_NO_PAD.encode(&key))
    }

    /// Encrypt file data
    pub fn encrypt_file(&self, data: &[u8], key_b64: &str) -> Result<Vec<u8>> {
        let key_bytes = URL_SAFE_NO_PAD.decode(key_b64)?;

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt file data
    pub fn decrypt_file(&self, encrypted: &[u8], key_b64: &str) -> Result<Vec<u8>> {
        if encrypted.len() < 12 {
            return Err(anyhow::anyhow!("Data too short"));
        }

        let key_bytes = URL_SAFE_NO_PAD.decode(key_b64)?;

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)?;

        let nonce = Nonce::from_slice(&encrypted[..12]);
        let ciphertext = &encrypted[12..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
    }

    /// Compute SHA-256 hash
    pub fn hash(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}

impl Default for CryptoEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let engine = CryptoEngine::new();
        engine.generate_identity().unwrap();

        let public_key = engine.get_public_key().unwrap();
        assert!(!public_key.is_empty());
    }

    #[test]
    fn test_encryption_decryption() {
        let alice = CryptoEngine::new();
        alice.generate_identity().unwrap();

        let bob = CryptoEngine::new();
        bob.generate_identity().unwrap();

        let alice_pub = alice.get_public_key().unwrap();
        let bob_pub = bob.get_public_key().unwrap();

        alice.establish_session("bob", &bob_pub).unwrap();
        bob.establish_session("alice", &alice_pub).unwrap();

        let plaintext = "Hello, Bob!";
        let encrypted = alice.encrypt_for("bob", plaintext).unwrap();
        let decrypted = bob.decrypt_from("alice", &encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }
}
