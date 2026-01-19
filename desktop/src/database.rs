//! Local SQLite database for PrivMsg Desktop

use crate::state::{
    Attachment, AuthSession, ChatMessage, Conversation, MessageStatus, MessageType,
};
use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("privmsg.db");
        let conn = Connection::open(&db_path)?;

        // Initialize schema
        conn.execute_batch(
            r#"
            -- Sessions
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY,
                token TEXT NOT NULL,
                device_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                expires_at INTEGER NOT NULL,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            -- Private keys (encrypted)
            CREATE TABLE IF NOT EXISTS keys (
                id INTEGER PRIMARY KEY,
                user_id TEXT NOT NULL,
                private_key TEXT NOT NULL,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            -- Conversations
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                peer_id TEXT NOT NULL UNIQUE,
                peer_name TEXT,
                peer_avatar TEXT,
                last_message TEXT,
                last_message_time INTEGER,
                unread_count INTEGER DEFAULT 0,
                is_muted INTEGER DEFAULT 0,
                is_pinned INTEGER DEFAULT 0,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            -- Messages
            CREATE TABLE IF NOT EXISTS messages (
                message_id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                message_type TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                status TEXT NOT NULL,
                is_outgoing INTEGER NOT NULL,
                attachment_file_id TEXT,
                attachment_file_name TEXT,
                attachment_file_size INTEGER,
                attachment_mime_type TEXT,
                attachment_duration_ms INTEGER,
                attachment_width INTEGER,
                attachment_height INTEGER,
                attachment_encryption_key TEXT,
                attachment_local_path TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (conversation_id) REFERENCES conversations(id)
            );

            -- Peer public keys cache
            CREATE TABLE IF NOT EXISTS peer_keys (
                user_id TEXT PRIMARY KEY,
                public_key TEXT NOT NULL,
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            -- Settings
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- Indices
            CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
            CREATE INDEX IF NOT EXISTS idx_conversations_updated ON conversations(updated_at);
            "#,
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ============= Sessions =============

    pub fn save_session(&self, session: &AuthSession) -> Result<()> {
        let conn = self.conn.lock();

        // Clear old sessions
        conn.execute("DELETE FROM sessions", [])?;

        conn.execute(
            "INSERT INTO sessions (token, device_id, user_id, expires_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                session.token,
                session.device_id,
                session.user_id,
                session.expires_at
            ],
        )?;

        Ok(())
    }

    pub fn get_session(&self) -> Option<AuthSession> {
        let conn = self.conn.lock();

        conn.query_row(
            "SELECT token, device_id, user_id, expires_at FROM sessions ORDER BY id DESC LIMIT 1",
            [],
            |row| {
                Ok(AuthSession {
                    token: row.get(0)?,
                    device_id: row.get(1)?,
                    user_id: row.get(2)?,
                    expires_at: row.get(3)?,
                })
            },
        )
        .ok()
    }

    pub fn clear_session(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sessions", [])?;
        Ok(())
    }

    // ============= Keys =============

    pub fn save_private_key(&self, user_id: &str, private_key: &str) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            "INSERT OR REPLACE INTO keys (user_id, private_key) VALUES (?1, ?2)",
            params![user_id, private_key],
        )?;

        Ok(())
    }

    pub fn get_private_key(&self, user_id: &str) -> Option<String> {
        let conn = self.conn.lock();

        conn.query_row(
            "SELECT private_key FROM keys WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )
        .ok()
    }

    pub fn save_peer_public_key(&self, user_id: &str, public_key: &str) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            "INSERT OR REPLACE INTO peer_keys (user_id, public_key, updated_at) VALUES (?1, ?2, strftime('%s', 'now'))",
            params![user_id, public_key],
        )?;

        Ok(())
    }

    pub fn get_peer_public_key(&self, user_id: &str) -> Option<String> {
        let conn = self.conn.lock();

        conn.query_row(
            "SELECT public_key FROM peer_keys WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )
        .ok()
    }

    // ============= Conversations =============

    pub fn save_conversation(&self, conv: &Conversation) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO conversations
            (id, peer_id, peer_name, peer_avatar, last_message, last_message_time,
             unread_count, is_muted, is_pinned, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%s', 'now'))
            "#,
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
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, peer_id, peer_name, peer_avatar, last_message, last_message_time,
                   unread_count, is_muted, is_pinned
            FROM conversations
            ORDER BY is_pinned DESC, last_message_time DESC
            "#,
        )?;

        let conversations = stmt
            .query_map([], |row| {
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
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(conversations)
    }

    pub fn update_conversation_last_message(
        &self,
        peer_id: &str,
        message: &str,
        timestamp: i64,
    ) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            r#"
            UPDATE conversations
            SET last_message = ?1, last_message_time = ?2, updated_at = strftime('%s', 'now')
            WHERE peer_id = ?3
            "#,
            params![message, timestamp, peer_id],
        )?;

        Ok(())
    }

    pub fn increment_unread_count(&self, peer_id: &str) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            "UPDATE conversations SET unread_count = unread_count + 1 WHERE peer_id = ?1",
            params![peer_id],
        )?;

        Ok(())
    }

    pub fn clear_unread_count(&self, peer_id: &str) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            "UPDATE conversations SET unread_count = 0 WHERE peer_id = ?1",
            params![peer_id],
        )?;

        Ok(())
    }

    // ============= Messages =============

    pub fn save_message(&self, msg: &ChatMessage) -> Result<()> {
        let conn = self.conn.lock();

        let message_type = match msg.message_type {
            MessageType::Text => "text",
            MessageType::Voice => "voice",
            MessageType::Video => "video",
            MessageType::Image => "image",
            MessageType::File => "file",
        };

        let status = match msg.status {
            MessageStatus::Pending => "pending",
            MessageStatus::Sent => "sent",
            MessageStatus::Delivered => "delivered",
            MessageStatus::Read => "read",
            MessageStatus::Failed => "failed",
        };

        let (
            att_file_id,
            att_file_name,
            att_file_size,
            att_mime_type,
            att_duration,
            att_width,
            att_height,
            att_key,
            att_path,
        ) = if let Some(ref att) = msg.attachment {
            (
                Some(att.file_id.clone()),
                Some(att.file_name.clone()),
                Some(att.file_size),
                Some(att.mime_type.clone()),
                att.duration_ms,
                att.width,
                att.height,
                att.encryption_key.clone(),
                att.local_path.clone(),
            )
        } else {
            (None, None, None, None, None, None, None, None, None)
        };

        conn.execute(
            r#"
            INSERT OR REPLACE INTO messages
            (message_id, conversation_id, sender_id, message_type, content, timestamp, status,
             is_outgoing, attachment_file_id, attachment_file_name, attachment_file_size,
             attachment_mime_type, attachment_duration_ms, attachment_width, attachment_height,
             attachment_encryption_key, attachment_local_path)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            "#,
            params![
                msg.message_id,
                msg.conversation_id,
                msg.sender_id,
                message_type,
                msg.content,
                msg.timestamp,
                status,
                msg.is_outgoing as i32,
                att_file_id,
                att_file_name,
                att_file_size,
                att_mime_type,
                att_duration,
                att_width,
                att_height,
                att_key,
                att_path,
            ],
        )?;

        // Update conversation
        self.update_conversation_last_message(&msg.conversation_id, &msg.content, msg.timestamp)?;

        Ok(())
    }

    pub fn get_messages(
        &self,
        conversation_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChatMessage>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r#"
            SELECT message_id, conversation_id, sender_id, message_type, content, timestamp,
                   status, is_outgoing, attachment_file_id, attachment_file_name,
                   attachment_file_size, attachment_mime_type, attachment_duration_ms,
                   attachment_width, attachment_height, attachment_encryption_key,
                   attachment_local_path
            FROM messages
            WHERE conversation_id = ?1
            ORDER BY timestamp ASC
            LIMIT ?2 OFFSET ?3
            "#,
        )?;

        let messages = stmt
            .query_map(params![conversation_id, limit, offset], |row| {
                let message_type = match row.get::<_, String>(3)?.as_str() {
                    "text" => MessageType::Text,
                    "voice" => MessageType::Voice,
                    "video" => MessageType::Video,
                    "image" => MessageType::Image,
                    "file" => MessageType::File,
                    _ => MessageType::Text,
                };

                let status = match row.get::<_, String>(6)?.as_str() {
                    "pending" => MessageStatus::Pending,
                    "sent" => MessageStatus::Sent,
                    "delivered" => MessageStatus::Delivered,
                    "read" => MessageStatus::Read,
                    "failed" => MessageStatus::Failed,
                    _ => MessageStatus::Pending,
                };

                let attachment = if let Some(file_id) = row.get::<_, Option<String>>(8)? {
                    Some(Attachment {
                        file_id,
                        file_name: row.get(9)?,
                        file_size: row.get(10)?,
                        mime_type: row.get(11)?,
                        duration_ms: row.get(12)?,
                        width: row.get(13)?,
                        height: row.get(14)?,
                        encryption_key: row.get(15)?,
                        local_path: row.get(16)?,
                    })
                } else {
                    None
                };

                Ok(ChatMessage {
                    message_id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    sender_id: row.get(2)?,
                    message_type,
                    content: row.get(4)?,
                    timestamp: row.get(5)?,
                    status,
                    is_outgoing: row.get::<_, i32>(7)? != 0,
                    attachment,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages)
    }

    pub fn update_message_status(&self, message_id: &str, status: MessageStatus) -> Result<()> {
        let conn = self.conn.lock();

        let status_str = match status {
            MessageStatus::Pending => "pending",
            MessageStatus::Sent => "sent",
            MessageStatus::Delivered => "delivered",
            MessageStatus::Read => "read",
            MessageStatus::Failed => "failed",
        };

        conn.execute(
            "UPDATE messages SET status = ?1 WHERE message_id = ?2",
            params![status_str, message_id],
        )?;

        Ok(())
    }

    // ============= Settings =============

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;

        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock();

        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok()
    }

    // ============= Cleanup =============

    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute_batch(
            r#"
            DELETE FROM sessions;
            DELETE FROM keys;
            DELETE FROM conversations;
            DELETE FROM messages;
            DELETE FROM peer_keys;
            DELETE FROM settings;
            "#,
        )?;

        Ok(())
    }
}
