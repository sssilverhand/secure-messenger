//! WebSocket connection management for PrivMsg Server

use dashmap::DashMap;
use tokio::sync::mpsc;
use crate::models::{WsServerMessage, PresenceStatus};

/// Represents an active WebSocket connection
#[derive(Clone)]
pub struct Connection {
    #[allow(dead_code)]
    pub user_id: String,
    pub device_id: String,
    pub sender: mpsc::UnboundedSender<WsServerMessage>,
}

/// Manages all active WebSocket connections
pub struct WebSocketManager {
    /// Map of user_id -> Vec<Connection> (multiple devices per user)
    connections: DashMap<String, Vec<Connection>>,
    /// Map of device_id -> user_id for quick lookup
    device_to_user: DashMap<String, String>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            device_to_user: DashMap::new(),
        }
    }

    /// Register a new connection
    pub fn register(&self, user_id: &str, device_id: &str, sender: mpsc::UnboundedSender<WsServerMessage>) {
        let connection = Connection {
            user_id: user_id.to_string(),
            device_id: device_id.to_string(),
            sender,
        };

        // Add to user's connections
        self.connections
            .entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(connection);

        // Map device to user
        self.device_to_user.insert(device_id.to_string(), user_id.to_string());

        tracing::info!("Connection registered: user={}, device={}", user_id, device_id);
    }

    /// Unregister a connection
    pub fn unregister(&self, device_id: &str) {
        if let Some((_, user_id)) = self.device_to_user.remove(device_id) {
            if let Some(mut connections) = self.connections.get_mut(&user_id) {
                connections.retain(|c| c.device_id != device_id);

                // If no more connections for this user, remove the entry
                if connections.is_empty() {
                    drop(connections);
                    self.connections.remove(&user_id);
                }
            }

            tracing::info!("Connection unregistered: user={}, device={}", user_id, device_id);
        }
    }

    /// Check if a user is online (has any active connections)
    pub fn is_user_online(&self, user_id: &str) -> bool {
        self.connections.get(user_id).map(|c| !c.is_empty()).unwrap_or(false)
    }

    /// Get number of online users
    pub fn online_user_count(&self) -> usize {
        self.connections.len()
    }

    /// Get all device IDs for a user
    pub fn get_user_devices(&self, user_id: &str) -> Vec<String> {
        self.connections
            .get(user_id)
            .map(|connections| connections.iter().map(|c| c.device_id.clone()).collect())
            .unwrap_or_default()
    }

    /// Send message to a specific user (all devices)
    pub fn send_to_user(&self, user_id: &str, message: WsServerMessage) {
        if let Some(connections) = self.connections.get(user_id) {
            for conn in connections.iter() {
                if let Err(e) = conn.sender.send(message.clone()) {
                    tracing::warn!("Failed to send to device {}: {}", conn.device_id, e);
                }
            }
        }
    }

    /// Send message to a specific device
    pub fn send_to_device(&self, device_id: &str, message: WsServerMessage) {
        if let Some(user_id) = self.device_to_user.get(device_id) {
            if let Some(connections) = self.connections.get(user_id.value()) {
                for conn in connections.iter() {
                    if conn.device_id == device_id {
                        if let Err(e) = conn.sender.send(message) {
                            tracing::warn!("Failed to send to device {}: {}", device_id, e);
                        }
                        return;
                    }
                }
            }
        }
    }

    /// Send message to all devices of a user except the specified one
    pub fn send_to_other_devices(&self, user_id: &str, exclude_device_id: &str, message: WsServerMessage) {
        if let Some(connections) = self.connections.get(user_id) {
            for conn in connections.iter() {
                if conn.device_id != exclude_device_id {
                    if let Err(e) = conn.sender.send(message.clone()) {
                        tracing::warn!("Failed to send to device {}: {}", conn.device_id, e);
                    }
                }
            }
        }
    }

    /// Broadcast presence change to all contacts
    pub fn broadcast_presence(&self, user_id: &str, status: PresenceStatus, contact_ids: &[String]) {
        let message = WsServerMessage::Presence {
            user_id: user_id.to_string(),
            status,
        };

        for contact_id in contact_ids {
            self.send_to_user(contact_id, message.clone());
        }
    }

    /// Broadcast user online notification
    pub fn broadcast_user_online(&self, user_id: &str, contact_ids: &[String]) {
        let message = WsServerMessage::UserOnline {
            user_id: user_id.to_string(),
        };

        for contact_id in contact_ids {
            self.send_to_user(contact_id, message.clone());
        }
    }

    /// Broadcast user offline notification
    pub fn broadcast_user_offline(&self, user_id: &str, contact_ids: &[String]) {
        let message = WsServerMessage::UserOffline {
            user_id: user_id.to_string(),
        };

        for contact_id in contact_ids {
            self.send_to_user(contact_id, message.clone());
        }
    }

    /// Get all online user IDs
    pub fn get_online_users(&self) -> Vec<String> {
        self.connections.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_management() {
        let manager = WebSocketManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();

        // Register connection
        manager.register("user1", "device1", tx.clone());
        assert!(manager.is_user_online("user1"));
        assert!(!manager.is_user_online("user2"));

        // Register another device for same user
        let (tx2, _rx2) = mpsc::unbounded_channel();
        manager.register("user1", "device2", tx2);
        assert_eq!(manager.get_user_devices("user1").len(), 2);

        // Unregister one device
        manager.unregister("device1");
        assert!(manager.is_user_online("user1"));
        assert_eq!(manager.get_user_devices("user1").len(), 1);

        // Unregister last device
        manager.unregister("device2");
        assert!(!manager.is_user_online("user1"));
    }
}
