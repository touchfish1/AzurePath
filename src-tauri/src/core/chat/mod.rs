mod store;

use crate::core::connection::{ConnectionManager, IncomingFrame};
use crate::types::chat::{Frame, StoredMessage};
use store::ChatStore;
use uuid::Uuid;

/// High-level chat operations used by the command layer.
pub struct ChatService {
    store: ChatStore,
}

impl ChatService {
    pub fn new() -> Result<Self, String> {
        let store = ChatStore::new()?;
        Ok(Self { store })
    }

    pub fn store(&self) -> &ChatStore {
        &self.store
    }

    /// Send a chat message to a specific peer.
    pub async fn send(
        &self,
        conn_mgr: &ConnectionManager,
        peer_id: &str,
        content: String,
        peer_name: &str,
        peer_ip: &str,
        peer_os: &str,
    ) -> Result<StoredMessage, String> {
        let msg_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let my_id = crate::core::discovery::my_id().await;

        let frame = Frame::ChatMsg {
            id: msg_id.clone(),
            from: my_id.clone(),
            from_name: crate::core::discovery::my_hostname().await,
            from_ip: "0.0.0.0".to_string(),
            from_os: std::env::consts::OS.to_string(),
            content: content.clone(),
            to: peer_id.to_string(),
            created_at: now.clone(),
        };

        conn_mgr.send(peer_id, &frame).await?;

        let stored = StoredMessage {
            id: msg_id,
            peer_id: peer_id.to_string(),
            peer_name: peer_name.to_string(),
            peer_ip: peer_ip.to_string(),
            peer_os: Some(peer_os.to_string()),
            content,
            is_broadcast: false,
            is_incoming: false,
            file_ref: None,
            created_at: now,
        };

        self.store.insert_message(&stored).ok();
        Ok(stored)
    }

    /// Broadcast a chat message to all connected peers.
    pub async fn broadcast(
        &self,
        conn_mgr: &ConnectionManager,
        content: String,
    ) -> Result<StoredMessage, String> {
        let msg_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let frame = Frame::ChatMsg {
            id: msg_id.clone(),
            from: crate::core::discovery::my_id().await,
            from_name: crate::core::discovery::my_hostname().await,
            from_ip: "0.0.0.0".to_string(),
            from_os: std::env::consts::OS.to_string(),
            content: content.clone(),
            to: "*".to_string(),
            created_at: now.clone(),
        };

        conn_mgr.broadcast(&frame).await;

        let stored = StoredMessage {
            id: msg_id,
            peer_id: "*".to_string(),
            peer_name: crate::core::discovery::my_hostname().await,
            peer_ip: "0.0.0.0".to_string(),
            peer_os: Some(std::env::consts::OS.to_string()),
            content,
            is_broadcast: true,
            is_incoming: false,
            file_ref: None,
            created_at: now,
        };

        self.store.insert_message(&stored).ok();
        Ok(stored)
    }

    /// Handle an incoming frame from a peer. Returns the stored message if it's a chat message.
    pub fn handle_incoming(&self, incoming: &IncomingFrame) -> Option<StoredMessage> {
        match &incoming.frame {
            Frame::ChatMsg {
                id,
                from,
                from_name,
                from_ip,
                from_os,
                content,
                to,
                created_at,
            } => {
                let stored = StoredMessage {
                    id: id.clone(),
                    peer_id: from.clone(),
                    peer_name: from_name.clone(),
                    peer_ip: from_ip.clone(),
                    peer_os: Some(from_os.clone()),
                    content: content.clone(),
                    is_broadcast: to == "*",
                    is_incoming: true,
                    file_ref: None,
                    created_at: created_at.clone(),
                };
                self.store.insert_message(&stored).ok();
                Some(stored)
            }
            _ => None,
        }
    }
}
