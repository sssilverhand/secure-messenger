//! Integration tests for PrivMsg Server

use reqwest::Client;
use serde_json::json;

const BASE_URL: &str = "http://localhost:8443";

#[tokio::test]
async fn test_health_check() {
    let client = Client::new();
    let response = client
        .get(format!("{}/health", BASE_URL))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success());
            let body: serde_json::Value = resp.json().await.unwrap();
            assert_eq!(body["status"], "ok");
        }
        Err(_) => {
            // Server not running, skip test
            println!("Server not running, skipping health check test");
        }
    }
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let client = Client::new();
    let response = client
        .post(format!("{}/api/v1/auth/login", BASE_URL))
        .json(&json!({
            "user_id": "invalid_user",
            "access_key": "invalid_key",
            "device_name": "Test Device",
            "device_type": "test",
            "device_public_key": "test_key"
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert_eq!(resp.status(), 401);
        }
        Err(_) => {
            println!("Server not running, skipping login test");
        }
    }
}

#[cfg(test)]
mod crypto_tests {
    use privmsg_server::crypto;

    #[test]
    fn test_user_id_generation() {
        let id = crypto::generate_user_id();
        assert_eq!(id.len(), 8);
        assert!(id.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_access_key_generation() {
        let key = crypto::generate_access_key();
        assert!(!key.is_empty());
        // Base64 URL encoded 32 bytes = 43 chars
        assert!(key.len() >= 40);
    }

    #[test]
    fn test_access_key_verification() {
        let key = crypto::generate_access_key();
        let hash = crypto::hash_access_key(&key);

        assert!(crypto::verify_access_key(&key, &hash));
        assert!(!crypto::verify_access_key("wrong_key", &hash));
    }

    #[test]
    fn test_session_token_generation() {
        let token = crypto::generate_session_token();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_file_id_generation() {
        let id1 = crypto::generate_file_id();
        let id2 = crypto::generate_file_id();

        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_turn_credentials() {
        let (username, credential) = crypto::generate_turn_credentials(
            "testuser",
            "secret",
            3600,
        );

        assert!(username.contains(':'));
        assert!(!credential.is_empty());
    }
}
