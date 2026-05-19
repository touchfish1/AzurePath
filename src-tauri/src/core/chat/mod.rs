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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::connection::ConnectionManager;
    use crate::core::connection::IncomingFrame;
    use crate::types::chat::Frame;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_service() -> ChatService {
        let store = store::ChatStore::new_test().expect("in-memory store");
        ChatService { store }
    }

    fn chat_frame(
        id: &str,
        from: &str,
        from_name: &str,
        from_ip: &str,
        from_os: &str,
        content: &str,
        to: &str,
        created_at: &str,
    ) -> Frame {
        Frame::ChatMsg {
            id: id.to_string(),
            from: from.to_string(),
            from_name: from_name.to_string(),
            from_ip: from_ip.to_string(),
            from_os: from_os.to_string(),
            content: content.to_string(),
            to: to.to_string(),
            created_at: created_at.to_string(),
        }
    }

    fn incoming(frame: Frame, peer_id: &str) -> IncomingFrame {
        IncomingFrame {
            peer_id: peer_id.to_string(),
            frame,
        }
    }

    // -----------------------------------------------------------------------
    // handle_incoming: ChatMsg frames
    // -----------------------------------------------------------------------

    #[test]
    fn test_handle_incoming_chat_msg_returns_some() {
        let svc = make_service();
        let f = chat_frame("m1", "peer-a", "Alice", "10.0.0.1", "linux", "Hi!", "*", "2025-01-01T00:00:00+00:00");
        let result = svc.handle_incoming(&incoming(f, "peer-a"));
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_incoming_chat_msg_field_mapping() {
        let svc = make_service();
        let f = chat_frame(
            "msg-abc",
            "sender-id",
            "SenderName",
            "192.168.1.50",
            "windows",
            "Hello, world!",
            "target-id",
            "2025-06-01T12:30:00+00:00",
        );
        let result = svc.handle_incoming(&incoming(f, "sender-id")).unwrap();

        assert_eq!(result.id, "msg-abc");
        assert_eq!(result.peer_id, "sender-id");
        assert_eq!(result.peer_name, "SenderName");
        assert_eq!(result.peer_ip, "192.168.1.50");
        assert_eq!(result.peer_os, Some("windows".to_string()));
        assert_eq!(result.content, "Hello, world!");
        assert!(!result.is_broadcast);
        assert!(result.is_incoming);
        assert!(result.file_ref.is_none());
        assert_eq!(result.created_at, "2025-06-01T12:30:00+00:00");
    }

    #[test]
    fn test_handle_incoming_broadcast_message() {
        let svc = make_service();
        let f = chat_frame("b1", "peer-b", "Broadcaster", "10.0.0.2", "macos", "Hello everyone!", "*", "2025-01-01T00:00:00+00:00");
        let result = svc.handle_incoming(&incoming(f, "peer-b")).unwrap();
        assert!(result.is_broadcast);
    }

    #[test]
    fn test_handle_incoming_direct_message_is_not_broadcast() {
        let svc = make_service();
        let f = chat_frame("d1", "peer-c", "Charlie", "10.0.0.3", "linux", "Hey you!", "specific-peer", "2025-01-01T00:00:00+00:00");
        let result = svc.handle_incoming(&incoming(f, "peer-c")).unwrap();
        assert!(!result.is_broadcast);
    }

    #[test]
    fn test_handle_incoming_empty_content() {
        let svc = make_service();
        let f = chat_frame("ec1", "peer-d", "D", "10.0.0.4", "linux", "", "target", "2025-01-01T00:00:00+00:00");
        let result = svc.handle_incoming(&incoming(f, "peer-d")).unwrap();
        assert_eq!(result.content, "");
    }

    #[test]
    fn test_handle_incoming_special_characters() {
        let svc = make_service();
        let special = "Line1\nLine2\tTabbed\nUnicode: \u{1F600} \u{00E9} \u{4F60}\u{597D}";
        let f = chat_frame("sc1", "peer-e", "E", "10.0.0.5", "linux", special, "*", "2025-01-01T00:00:00+00:00");
        let result = svc.handle_incoming(&incoming(f, "peer-e")).unwrap();
        assert_eq!(result.content, special);
    }

    #[test]
    fn test_handle_incoming_very_long_content() {
        let svc = make_service();
        let long = "x".repeat(50_000);
        let f = chat_frame("long1", "peer-f", "F", "10.0.0.6", "linux", &long, "*", "2025-01-01T00:00:00+00:00");
        let result = svc.handle_incoming(&incoming(f, "peer-f")).unwrap();
        assert_eq!(result.content.len(), 50_000);
    }

    #[test]
    fn test_handle_incoming_ipv6_address() {
        let svc = make_service();
        let f = chat_frame("ipv6", "peer-g", "G", "::1", "linux", "IPv6 test", "target", "2025-01-01T00:00:00+00:00");
        let result = svc.handle_incoming(&incoming(f, "peer-g")).unwrap();
        assert_eq!(result.peer_ip, "::1");
    }

    #[test]
    fn test_handle_incoming_unknown_os() {
        let svc = make_service();
        let f = chat_frame("os1", "peer-h", "H", "10.0.0.8", "unknown-os-v99", "Test", "*", "2025-01-01T00:00:00+00:00");
        let result = svc.handle_incoming(&incoming(f, "peer-h")).unwrap();
        assert_eq!(result.peer_os, Some("unknown-os-v99".to_string()));
    }

    // -----------------------------------------------------------------------
    // handle_incoming: Non-chat frames
    // -----------------------------------------------------------------------

    #[test]
    fn test_handle_incoming_hello_returns_none() {
        let svc = make_service();
        let f = Frame::Hello { id: "some-peer".into() };
        assert!(svc.handle_incoming(&incoming(f, "some-peer")).is_none());
    }

    #[test]
    fn test_handle_incoming_system_returns_none() {
        let svc = make_service();
        let f = Frame::System { content: "system message".into() };
        assert!(svc.handle_incoming(&incoming(f, "system")).is_none());
    }

    #[test]
    fn test_handle_incoming_ping_returns_none() {
        let svc = make_service();
        assert!(svc.handle_incoming(&incoming(Frame::Ping, "pinger")).is_none());
    }

    #[test]
    fn test_handle_incoming_pong_returns_none() {
        let svc = make_service();
        assert!(svc.handle_incoming(&incoming(Frame::Pong, "ponger")).is_none());
    }

    #[test]
    fn test_handle_incoming_file_frames_return_none() {
        let svc = make_service();

        let frames: Vec<Frame> = vec![
            Frame::FileRequest {
                file_id: "f1".into(),
                filename: "test.txt".into(),
                size: 100,
                from: "peer".into(),
            },
            Frame::FileResponse {
                file_id: "f1".into(),
                accepted: true,
                data_port: 12345,
            },
            Frame::FileProgress {
                file_id: "f1".into(),
                received: 50,
                total: 100,
                speed: 1.5,
            },
            Frame::FileComplete { file_id: "f1".into() },
            Frame::FileAck { file_id: "f1".into() },
        ];

        for f in frames {
            let result = svc.handle_incoming(&incoming(f, "peer-x"));
            assert!(result.is_none(), "Expected None for file-related frame");
        }
    }

    #[test]
    fn test_handle_incoming_clipboard_sync_returns_none() {
        let svc = make_service();
        let f = Frame::ClipboardSync { entries: vec![] };
        assert!(svc.handle_incoming(&incoming(f, "peer-y")).is_none());
    }

    // -----------------------------------------------------------------------
    // handle_incoming: side effects (message storage)
    // -----------------------------------------------------------------------

    #[test]
    fn test_handle_incoming_stores_message_in_store() {
        let svc = make_service();
        let f = chat_frame("store1", "peer-z", "Zed", "10.0.0.9", "linux", "Store check", "*", "2025-01-01T00:00:00+00:00");
        svc.handle_incoming(&incoming(f, "peer-z"));

        let stored = svc.store.get_messages(None, 10).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, "store1");
    }

    #[test]
    fn test_handle_incoming_stores_multiple_messages() {
        let svc = make_service();
        for i in 0..5 {
            let f = chat_frame(
                &format!("multi-{}", i),
                "peer-m",
                "Multi",
                "10.0.0.10",
                "linux",
                &format!("msg {}", i),
                "*",
                &format!("2025-01-01T00:00:{:02}+00:00", i),
            );
            svc.handle_incoming(&incoming(f, "peer-m"));
        }

        let stored = svc.store.get_messages(None, 10).unwrap();
        assert_eq!(stored.len(), 5);
    }

    #[test]
    fn test_handle_incoming_non_chat_does_not_store() {
        let svc = make_service();
        svc.handle_incoming(&incoming(Frame::Ping, "pinger"));
        svc.handle_incoming(&incoming(Frame::Pong, "ponger"));
        svc.handle_incoming(&incoming(Frame::System { content: "sys".into() }, "syspeer"));

        let stored = svc.store.get_messages(None, 10).unwrap();
        assert!(stored.is_empty(), "non-chat frames should not create stored messages");
    }

    // -----------------------------------------------------------------------
    // send (error path — no peer connected)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_send_returns_error_when_peer_not_connected() {
        let svc = make_service();
        let conn_mgr = ConnectionManager::new();
        let result = svc
            .send(&conn_mgr, "nonexistent-peer", "Hello".to_string(), "SomePeer", "10.0.0.1", "linux")
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("No connection to peer"),
            "Expected connection error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_send_does_not_store_on_network_error() {
        let svc = make_service();
        let conn_mgr = ConnectionManager::new();
        let _ = svc
            .send(&conn_mgr, "ghost", "should not store".to_string(), "Ghost", "10.0.0.2", "linux")
            .await;

        // No message should be stored since the send failed
        let stored = svc.store.get_messages(None, 10).unwrap();
        assert!(stored.is_empty(), "No message should be stored on send failure");
    }

    #[tokio::test]
    async fn test_send_empty_content_fails_with_same_error() {
        let svc = make_service();
        let conn_mgr = ConnectionManager::new();
        let result = svc
            .send(&conn_mgr, "target-peer", String::new(), "Target", "10.0.0.3", "linux")
            .await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // broadcast (succeeds even with no peers)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_broadcast_succeeds_with_no_peers() {
        let svc = make_service();
        let conn_mgr = ConnectionManager::new();
        let result = svc.broadcast(&conn_mgr, "Hello everyone!".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_returns_correct_message() {
        let svc = make_service();
        let conn_mgr = ConnectionManager::new();
        let msg = svc.broadcast(&conn_mgr, "Broadcast test".to_string()).await.unwrap();

        assert_eq!(msg.content, "Broadcast test");
        assert_eq!(msg.peer_id, "*");
        assert!(msg.is_broadcast);
        assert!(!msg.is_incoming);
        assert!(msg.peer_os.is_some());
        assert!(msg.file_ref.is_none());
        assert!(!msg.id.is_empty());
        assert!(!msg.created_at.is_empty());
    }

    #[tokio::test]
    async fn test_broadcast_stores_message() {
        let svc = make_service();
        let conn_mgr = ConnectionManager::new();
        let msg = svc.broadcast(&conn_mgr, "Stored broadcast".to_string()).await.unwrap();

        let stored = svc.store.get_messages(None, 10).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, msg.id);
        assert_eq!(stored[0].content, "Stored broadcast");
    }

    #[tokio::test]
    async fn test_broadcast_empty_content() {
        let svc = make_service();
        let conn_mgr = ConnectionManager::new();
        let msg = svc.broadcast(&conn_mgr, String::new()).await.unwrap();
        assert_eq!(msg.content, "");
    }

    #[tokio::test]
    async fn test_broadcast_unicode_content() {
        let svc = make_service();
        let conn_mgr = ConnectionManager::new();
        let unicode = "Broadcast \u{1F604} with 中文 and emoji!";
        let msg = svc.broadcast(&conn_mgr, unicode.to_string()).await.unwrap();
        assert_eq!(msg.content, unicode);
    }

    // -----------------------------------------------------------------------
    // store accessor
    // -----------------------------------------------------------------------

    #[test]
    fn test_store_accessor_returns_same_instance() {
        let svc = make_service();
        let store_ref = svc.store();
        let stored = store_ref.get_messages(None, 10).unwrap();
        assert!(stored.is_empty());
    }
}
