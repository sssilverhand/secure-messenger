//! Database storage layer for PrivMsg Server

use chrono::{DateTime, Duration, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;

use crate::crypto;
use crate::models::*;

pub struct Storage {
    pool: Pool<Sqlite>,
}

impl Storage {
    pub async fn new(database_path: &str) -> anyhow::Result<Self> {
        // Ensure directory exists
        if let Some(parent) = Path::new(database_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let database_url = format!("sqlite:{}?mode=rwc", database_path);

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;

        let storage = Self { pool };
        storage.initialize_schema().await?;

        Ok(storage)
    }

    async fn initialize_schema(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                user_id TEXT PRIMARY KEY,
                key_hash TEXT NOT NULL,
                display_name TEXT,
                avatar_file_id TEXT,
                public_key TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_seen_at TEXT,
                is_active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS devices (
                device_id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                device_name TEXT NOT NULL,
                device_type TEXT NOT NULL,
                push_token TEXT,
                public_key TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_active_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS sessions (
                token_hash TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT NOT NULL,
                is_valid INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
                FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS pending_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT NOT NULL UNIQUE,
                sender_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                recipient_device_id TEXT,
                encrypted_content TEXT NOT NULL,
                message_type TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT NOT NULL,
                FOREIGN KEY (sender_id) REFERENCES users(user_id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS files (
                file_id TEXT PRIMARY KEY,
                uploader_id TEXT NOT NULL,
                file_name TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                mime_type TEXT NOT NULL,
                encryption_key_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT NOT NULL,
                download_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (uploader_id) REFERENCES users(user_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_devices_user ON devices(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_pending_recipient ON pending_messages(recipient_id);
            CREATE INDEX IF NOT EXISTS idx_pending_expires ON pending_messages(expires_at);
            CREATE INDEX IF NOT EXISTS idx_files_expires ON files(expires_at);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========================================================================
    // User Operations
    // ========================================================================

    pub async fn create_user(&self, user_id: &str, key_hash: &str) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO users (user_id, key_hash, created_at) VALUES (?, ?, datetime('now'))",
        )
        .bind(user_id)
        .bind(key_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user(&self, user_id: &str) -> anyhow::Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT user_id, key_hash, display_name, avatar_file_id, public_key,
                    created_at, last_seen_at, is_active
             FROM users WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn verify_user_credentials(&self, user_id: &str, access_key: &str) -> anyhow::Result<bool> {
        let user = self.get_user(user_id).await?;

        match user {
            Some(u) if u.is_active => Ok(crypto::verify_access_key(access_key, &u.key_hash)),
            _ => Ok(false),
        }
    }

    pub async fn update_user_profile(
        &self,
        user_id: &str,
        display_name: Option<&str>,
        avatar_file_id: Option<&str>,
        public_key: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE users SET
                display_name = COALESCE(?, display_name),
                avatar_file_id = COALESCE(?, avatar_file_id),
                public_key = COALESCE(?, public_key)
             WHERE user_id = ?",
        )
        .bind(display_name)
        .bind(avatar_file_id)
        .bind(public_key)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_user_last_seen(&self, user_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE users SET last_seen_at = datetime('now') WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn deactivate_user(&self, user_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE users SET is_active = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Invalidate all sessions
        sqlx::query("UPDATE sessions SET is_valid = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn list_users(&self) -> anyhow::Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT user_id, key_hash, display_name, avatar_file_id, public_key,
                    created_at, last_seen_at, is_active
             FROM users ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    pub async fn delete_user(&self, user_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM users WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Device Operations
    // ========================================================================

    pub async fn create_device(
        &self,
        user_id: &str,
        device_name: &str,
        device_type: &str,
        public_key: &str,
    ) -> anyhow::Result<String> {
        let device_id = crypto::generate_device_id();

        sqlx::query(
            "INSERT INTO devices (device_id, user_id, device_name, device_type, public_key, created_at, last_active_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))",
        )
        .bind(&device_id)
        .bind(user_id)
        .bind(device_name)
        .bind(device_type)
        .bind(public_key)
        .execute(&self.pool)
        .await?;

        Ok(device_id)
    }

    pub async fn get_device(&self, device_id: &str) -> anyhow::Result<Option<Device>> {
        let device = sqlx::query_as::<_, Device>(
            "SELECT device_id, user_id, device_name, device_type, push_token, public_key,
                    created_at, last_active_at
             FROM devices WHERE device_id = ?",
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(device)
    }

    pub async fn list_user_devices(&self, user_id: &str) -> anyhow::Result<Vec<Device>> {
        let devices = sqlx::query_as::<_, Device>(
            "SELECT device_id, user_id, device_name, device_type, push_token, public_key,
                    created_at, last_active_at
             FROM devices WHERE user_id = ? ORDER BY last_active_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(devices)
    }

    pub async fn update_device_activity(&self, device_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE devices SET last_active_at = datetime('now') WHERE device_id = ?")
            .bind(device_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_device(&self, device_id: &str) -> anyhow::Result<()> {
        // Invalidate all sessions for this device
        sqlx::query("UPDATE sessions SET is_valid = 0 WHERE device_id = ?")
            .bind(device_id)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM devices WHERE device_id = ?")
            .bind(device_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Session Operations
    // ========================================================================

    pub async fn create_session(
        &self,
        user_id: &str,
        device_id: &str,
        token: &str,
        ttl_hours: i64,
    ) -> anyhow::Result<DateTime<Utc>> {
        let token_hash = crypto::hash_access_key(token);
        let expires_at = Utc::now() + Duration::hours(ttl_hours);

        sqlx::query(
            "INSERT INTO sessions (token_hash, user_id, device_id, created_at, expires_at, is_valid)
             VALUES (?, ?, ?, datetime('now'), ?, 1)",
        )
        .bind(&token_hash)
        .bind(user_id)
        .bind(device_id)
        .bind(expires_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(expires_at)
    }

    pub async fn validate_session(&self, token: &str) -> anyhow::Result<Option<Session>> {
        let token_hash = crypto::hash_access_key(token);

        let session = sqlx::query_as::<_, Session>(
            "SELECT token_hash, user_id, device_id, created_at, expires_at, is_valid
             FROM sessions
             WHERE token_hash = ? AND is_valid = 1 AND expires_at > datetime('now')",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    pub async fn invalidate_session(&self, token: &str) -> anyhow::Result<()> {
        let token_hash = crypto::hash_access_key(token);

        sqlx::query("UPDATE sessions SET is_valid = 0 WHERE token_hash = ?")
            .bind(&token_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn invalidate_all_user_sessions(&self, user_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE sessions SET is_valid = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Message Operations
    // ========================================================================

    pub async fn store_pending_message(&self, envelope: &MessageEnvelope, ttl_hours: i64) -> anyhow::Result<()> {
        let expires_at = Utc::now() + Duration::hours(ttl_hours);

        sqlx::query(
            "INSERT INTO pending_messages
             (message_id, sender_id, recipient_id, recipient_device_id, encrypted_content, message_type, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'), ?)",
        )
        .bind(&envelope.message_id)
        .bind(&envelope.sender_id)
        .bind(&envelope.recipient_id)
        .bind(&envelope.recipient_device_id)
        .bind(&envelope.encrypted_content)
        .bind(envelope.message_type.to_string())
        .bind(expires_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_pending_messages(&self, user_id: &str, device_id: Option<&str>) -> anyhow::Result<Vec<PendingMessage>> {
        let messages = if let Some(did) = device_id {
            sqlx::query_as::<_, PendingMessage>(
                "SELECT id, message_id, sender_id, recipient_id, recipient_device_id,
                        encrypted_content, message_type, created_at, expires_at
                 FROM pending_messages
                 WHERE recipient_id = ? AND (recipient_device_id IS NULL OR recipient_device_id = ?)
                 AND expires_at > datetime('now')
                 ORDER BY created_at ASC",
            )
            .bind(user_id)
            .bind(did)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, PendingMessage>(
                "SELECT id, message_id, sender_id, recipient_id, recipient_device_id,
                        encrypted_content, message_type, created_at, expires_at
                 FROM pending_messages
                 WHERE recipient_id = ? AND expires_at > datetime('now')
                 ORDER BY created_at ASC",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(messages)
    }

    pub async fn delete_pending_messages(&self, message_ids: &[String]) -> anyhow::Result<()> {
        for message_id in message_ids {
            sqlx::query("DELETE FROM pending_messages WHERE message_id = ?")
                .bind(message_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    pub async fn count_pending_messages(&self) -> anyhow::Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pending_messages")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    // ========================================================================
    // File Operations
    // ========================================================================

    pub async fn create_file_metadata(
        &self,
        uploader_id: &str,
        file_name: &str,
        file_size: i64,
        mime_type: &str,
        encryption_key_hash: &str,
        ttl_hours: i64,
    ) -> anyhow::Result<String> {
        let file_id = crypto::generate_file_id();
        let expires_at = Utc::now() + Duration::hours(ttl_hours);

        sqlx::query(
            "INSERT INTO files
             (file_id, uploader_id, file_name, file_size, mime_type, encryption_key_hash, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'), ?)",
        )
        .bind(&file_id)
        .bind(uploader_id)
        .bind(file_name)
        .bind(file_size)
        .bind(mime_type)
        .bind(encryption_key_hash)
        .bind(expires_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(file_id)
    }

    pub async fn get_file_metadata(&self, file_id: &str) -> anyhow::Result<Option<FileMetadata>> {
        let file = sqlx::query_as::<_, FileMetadata>(
            "SELECT file_id, uploader_id, file_name, file_size, mime_type,
                    encryption_key_hash, created_at, expires_at, download_count
             FROM files WHERE file_id = ? AND expires_at > datetime('now')",
        )
        .bind(file_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(file)
    }

    pub async fn increment_download_count(&self, file_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE files SET download_count = download_count + 1 WHERE file_id = ?")
            .bind(file_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_file_metadata(&self, file_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM files WHERE file_id = ?")
            .bind(file_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn count_files(&self) -> anyhow::Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    pub async fn get_total_file_size(&self) -> anyhow::Result<i64> {
        let size: (i64,) = sqlx::query_as("SELECT COALESCE(SUM(file_size), 0) FROM files")
            .fetch_one(&self.pool)
            .await?;

        Ok(size.0)
    }

    // ========================================================================
    // Cleanup Operations
    // ========================================================================

    pub async fn cleanup_expired(&self) -> anyhow::Result<(i64, i64)> {
        // Get expired file IDs before deleting metadata (for potential file system cleanup)
        let _expired_files: Vec<(String,)> = sqlx::query_as(
            "SELECT file_id FROM files WHERE expires_at <= datetime('now')",
        )
        .fetch_all(&self.pool)
        .await?;

        // Delete expired messages
        let messages_result = sqlx::query(
            "DELETE FROM pending_messages WHERE expires_at <= datetime('now')",
        )
        .execute(&self.pool)
        .await?;

        // Delete expired file metadata
        let files_result = sqlx::query(
            "DELETE FROM files WHERE expires_at <= datetime('now')",
        )
        .execute(&self.pool)
        .await?;

        // Delete expired sessions
        sqlx::query("DELETE FROM sessions WHERE expires_at <= datetime('now') OR is_valid = 0")
            .execute(&self.pool)
            .await?;

        Ok((messages_result.rows_affected() as i64, files_result.rows_affected() as i64))
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    pub async fn get_stats(&self) -> anyhow::Result<ServerStats> {
        let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let active_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = 1")
            .fetch_one(&self.pool)
            .await?;

        let pending_messages = self.count_pending_messages().await?;
        let stored_files = self.count_files().await?;
        let storage_bytes = self.get_total_file_size().await?;

        Ok(ServerStats {
            total_users: total_users.0,
            active_users: active_users.0,
            online_users: 0, // Will be set by WebSocket manager
            pending_messages,
            stored_files,
            storage_used_mb: storage_bytes as f64 / (1024.0 * 1024.0),
        })
    }
}
