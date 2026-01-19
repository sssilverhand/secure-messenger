//! Local storage using SQLite

use crate::error::{Error, Result};
use crate::models::*;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub struct LocalStorage {
    conn: Mutex<Connection>,
}

impl LocalStorage {
    pub fn new(data_dir: &str) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = Path::new(data_dir).join("privmsg.db");
        let conn = Connection::open(db_path)?;

        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_schema()?;

        Ok(storage)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                peer_id TEXT NOT NULL,
                peer_name TEXT,
                peer_avatar TEXT,
                last_message TEXT,
                last_message_time INTEGER,
                unread_count INTEGER NOT NULL DEFAULT 0,
                is_muted INTEGER NOT NULL DEFAULT 0,
                is_pinned INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                message_id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                message_type TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                status TEXT NOT NULL,
                attachment_json TEXT,
                is_outgoing INTEGER NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id)
            );

            CREATE TABLE IF NOT EXISTS users (
                user_id TEXT PRIMARY KEY,
                display_name TEXT,
                avatar_file_id TEXT,
                public_key TEXT,
                last_seen_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS session_keys (
                peer_id TEXT PRIMARY KEY,
                shared_secret TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
            "#,
        )?;

        Ok(())
    }

    // ========================================================================
    // Settings
    // ========================================================================

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok()
    }

    pub fn delete_setting(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    // ========================================================================
    // Session
    // ========================================================================

    pub fn save_session(&self, session: &AuthSession) -> Result<()> {
        self.save_setting("token", &session.token)?;
        self.save_setting("device_id", &session.device_id)?;
        self.save_setting("current_user_id", &session.user_id)?;
        self.save_setting("expires_at", &session.expires_at.to_string())?;
        Ok(())
    }

    pub fn get_session(&self) -> Option<AuthSession> {
        let token = self.get_setting("token")?;
        let device_id = self.get_setting("device_id")?;
        let user_id = self.get_setting("current_user_id")?;
        let expires_at = self.get_setting("expires_at")?.parse().ok()?;

        Some(AuthSession {
            token,
            device_id,
            user_id,
            expires_at,
        })
    }

    pub fn clear_session(&self) -> Result<()> {
        self.delete_setting("token")?;
        self.delete_setting("device_id")?;
        self.delete_setting("current_user_id")?;
        self.delete_setting("expires_at")?;
        Ok(())
    }

    // ========================================================================
    // Conversations
    // ========================================================================

    pub fn save_conversation(&self, conv: &Conversation) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT OR REPLACE INTO conversations
               (id, peer_id, peer_name, peer_avatar, last_message, last_message_time, unread_count, is_muted, is_pinned)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                conv.id,
                conv.peer_id,
                conv.peer_name,
                conv.peer_avatar,
                conv.last_message,
                conv.last_message_time,
                conv.unread_count,
                conv.is_muted as i32,
                conv.is_pinned as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_conversations(&self) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT id, peer_id, peer_name, peer_avatar, last_message, last_message_time,
                      unread_count, is_muted, is_pinned
               FROM conversations
               ORDER BY is_pinned DESC, last_message_time DESC"#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                peer_id: row.get(1)?,
                peer_name: row.get(2)?,
                peer_avatar: row.get(3)?,
                last_message: row.get(4)?,
                last_message_time: row.get(5)?,
                unread_count: row.get(6)?,
                is_muted: row.get::<_, i32>(7)? != 0,
                is_pinned: row.get::<_, i32>(8)? != 0,
            })
        })?;

        let mut conversations = Vec::new();
        for row in rows {
            conversations.push(row?);
        }

        Ok(conversations)
    }

    pub fn get_conversation(&self, id: &str) -> Result<Option<Conversation>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            r#"SELECT id, peer_id, peer_name, peer_avatar, last_message, last_message_time,
                      unread_count, is_muted, is_pinned
               FROM conversations WHERE id = ?1"#,
            params![id],
            |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    peer_id: row.get(1)?,
                    peer_name: row.get(2)?,
                    peer_avatar: row.get(3)?,
                    last_message: row.get(4)?,
                    last_message_time: row.get(5)?,
                    unread_count: row.get(6)?,
                    is_muted: row.get::<_, i32>(7)? != 0,
                    is_pinned: row.get::<_, i32>(8)? != 0,
                })
            },
        );

        match result {
            Ok(conv) => Ok(Some(conv)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn update_unread_count(&self, conversation_id: &str, count: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE conversations SET unread_count = ?1 WHERE id = ?2",
            params![count, conversation_id],
        )?;
        Ok(())
    }

    pub fn delete_conversation(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM messages WHERE conversation_id = ?1", params![id])?;
        conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ========================================================================
    // Messages
    // ========================================================================

    pub fn save_message(&self, msg: &Message) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let attachment_json = msg
            .attachment
            .as_ref()
            .map(|a| serde_json::to_string(a).unwrap_or_default());

        conn.execute(
            r#"INSERT OR REPLACE INTO messages
               (message_id, conversation_id, sender_id, message_type, content, timestamp, status, attachment_json, is_outgoing)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                msg.message_id,
                msg.conversation_id,
                msg.sender_id,
                format!("{:?}", msg.message_type).to_lowercase(),
                msg.content,
                msg.timestamp,
                format!("{:?}", msg.status).to_lowercase(),
                attachment_json,
                msg.is_outgoing as i32,
            ],
        )?;

        // Update conversation
        conn.execute(
            r#"INSERT OR REPLACE INTO conversations (id, peer_id, last_message, last_message_time, unread_count, is_muted, is_pinned)
               VALUES (?1, ?1, ?2, ?3,
                       COALESCE((SELECT unread_count FROM conversations WHERE id = ?1), 0) + ?4,
                       COALESCE((SELECT is_muted FROM conversations WHERE id = ?1), 0),
                       COALESCE((SELECT is_pinned FROM conversations WHERE id = ?1), 0))"#,
            params![
                msg.conversation_id,
                if msg.content.len() > 50 { &msg.content[..50] } else { &msg.content },
                msg.timestamp,
                if msg.is_outgoing { 0 } else { 1 },
            ],
        )?;

        Ok(())
    }

    pub fn get_messages(&self, conversation_id: &str, limit: i64, offset: i64) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT message_id, conversation_id, sender_id, message_type, content,
                      timestamp, status, attachment_json, is_outgoing
               FROM messages
               WHERE conversation_id = ?1
               ORDER BY timestamp DESC
               LIMIT ?2 OFFSET ?3"#,
        )?;

        let rows = stmt.query_map(params![conversation_id, limit, offset], |row| {
            let type_str: String = row.get(3)?;
            let status_str: String = row.get(6)?;
            let attachment_json: Option<String> = row.get(7)?;

            Ok(Message {
                message_id: row.get(0)?,
                conversation_id: row.get(1)?,
                sender_id: row.get(2)?,
                message_type: match type_str.as_str() {
                    "voice" => MessageType::Voice,
                    "video" => MessageType::Video,
                    "image" => MessageType::Image,
                    "file" => MessageType::File,
                    _ => MessageType::Text,
                },
                content: row.get(4)?,
                timestamp: row.get(5)?,
                status: match status_str.as_str() {
                    "pending" => MessageStatus::Pending,
                    "delivered" => MessageStatus::Delivered,
                    "read" => MessageStatus::Read,
                    "failed" => MessageStatus::Failed,
                    _ => MessageStatus::Sent,
                },
                attachment: attachment_json.and_then(|j| serde_json::from_str(&j).ok()),
                is_outgoing: row.get::<_, i32>(8)? != 0,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        messages.reverse(); // Oldest first

        Ok(messages)
    }

    pub fn update_message_status(&self, message_id: &str, status: MessageStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE messages SET status = ?1 WHERE message_id = ?2",
            params![format!("{:?}", status).to_lowercase(), message_id],
        )?;
        Ok(())
    }

    pub fn delete_message(&self, message_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM messages WHERE message_id = ?1", params![message_id])?;
        Ok(())
    }

    // ========================================================================
    // Users cache
    // ========================================================================

    pub fn save_user(&self, user: &User) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT OR REPLACE INTO users (user_id, display_name, avatar_file_id, public_key, last_seen_at)
               VALUES (?1, ?2, ?3, ?4, ?5)"#,
            params![
                user.user_id,
                user.display_name,
                user.avatar_file_id,
                user.public_key,
                user.last_seen_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_user(&self, user_id: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT user_id, display_name, avatar_file_id, public_key, last_seen_at FROM users WHERE user_id = ?1",
            params![user_id],
            |row| {
                Ok(User {
                    user_id: row.get(0)?,
                    display_name: row.get(1)?,
                    avatar_file_id: row.get(2)?,
                    public_key: row.get(3)?,
                    last_seen_at: row.get(4)?,
                })
            },
        );

        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ========================================================================
    // Storage management
    // ========================================================================

    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            DELETE FROM messages;
            DELETE FROM conversations;
            DELETE FROM users;
            DELETE FROM settings;
            DELETE FROM session_keys;
            "#,
        )?;
        Ok(())
    }

    pub fn get_storage_size(&self) -> Result<u64> {
        // Approximate based on page count
        let conn = self.conn.lock().unwrap();
        let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        Ok((page_count * page_size) as u64)
    }
}
