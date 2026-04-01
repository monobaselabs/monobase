//! WebSocket service for real-time communication.
//! Manages user connections and channel subscriptions.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

/// WebSocket service managing connections and channels.
pub struct WebSocketService {
    /// User ID → broadcast sender for direct messages.
    user_channels: Arc<Mutex<HashMap<String, broadcast::Sender<WsMessage>>>>,
    /// Channel ID → broadcast sender for channel messages.
    channels: Arc<Mutex<HashMap<String, broadcast::Sender<WsMessage>>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    pub event: String,
    pub payload: serde_json::Value,
}

impl WebSocketService {
    pub fn new() -> Self {
        Self {
            user_channels: Arc::new(Mutex::new(HashMap::new())),
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a user connection. Returns a receiver for messages.
    pub async fn register_user(&self, user_id: &str) -> broadcast::Receiver<WsMessage> {
        let mut users = self.user_channels.lock().await;
        let (tx, rx) = broadcast::channel(100);
        users.insert(user_id.to_string(), tx);
        rx
    }

    /// Unregister a user connection.
    pub async fn unregister_user(&self, user_id: &str) {
        let mut users = self.user_channels.lock().await;
        users.remove(user_id);
    }

    /// Send a message to a specific user.
    pub async fn publish_to_user(&self, user_id: &str, event: &str, payload: serde_json::Value) -> bool {
        let users = self.user_channels.lock().await;
        if let Some(tx) = users.get(user_id) {
            tx.send(WsMessage {
                event: event.to_string(),
                payload,
            })
            .is_ok()
        } else {
            false
        }
    }

    /// Subscribe to a channel. Returns a receiver.
    pub async fn subscribe_channel(&self, channel_id: &str) -> broadcast::Receiver<WsMessage> {
        let mut channels = self.channels.lock().await;
        let entry = channels
            .entry(channel_id.to_string())
            .or_insert_with(|| broadcast::channel(100).0);
        entry.subscribe()
    }

    /// Publish to a channel.
    pub async fn publish_to_channel(&self, channel_id: &str, event: &str, payload: serde_json::Value) {
        let channels = self.channels.lock().await;
        if let Some(tx) = channels.get(channel_id) {
            let _ = tx.send(WsMessage {
                event: event.to_string(),
                payload,
            });
        }
    }

    /// Get connection stats.
    pub async fn stats(&self) -> (usize, usize) {
        let users = self.user_channels.lock().await;
        let channels = self.channels.lock().await;
        (users.len(), channels.len())
    }
}

impl Default for WebSocketService {
    fn default() -> Self {
        Self::new()
    }
}
