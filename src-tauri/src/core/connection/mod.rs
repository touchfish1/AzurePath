mod protocol;

use crate::types::chat::Frame;
use protocol::{read_frame, write_frame};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};

pub const LISTEN_PORT: u16 = 42070;

pub struct PeerConnection {
    pub peer_id: String,
    pub addr: String,
    pub writer: Mutex<OwnedWriteHalf>,
}

/// Helper to extract peer Arcs so the connections lock is not held across async I/O.
fn collect_peers(
    map: &HashMap<String, Arc<PeerConnection>>,
) -> Vec<Arc<PeerConnection>> {
    map.values().cloned().collect()
}

#[derive(Debug, Clone)]
pub struct IncomingFrame {
    pub peer_id: String,
    pub frame: Frame,
}

pub struct ConnectionManager {
    connections: Arc<Mutex<HashMap<String, Arc<PeerConnection>>>>,
    frame_tx: broadcast::Sender<IncomingFrame>,
    running: Arc<AtomicBool>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let (frame_tx, _) = broadcast::channel(256);
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            frame_tx,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<IncomingFrame> {
        self.frame_tx.subscribe()
    }

    pub async fn start_listener(self: &Arc<Self>) -> Result<(), String> {
        let addr = format!("0.0.0.0:{}", LISTEN_PORT);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("Failed to bind TCP listener: {}", e))?;
        info!("[conn] TCP listener started on {}", addr);

        let this = self.clone();
        tokio::spawn(async move {
            loop {
                if !this.running.load(Ordering::SeqCst) {
                    break;
                }
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let peer_addr_str = peer_addr.to_string();
                        info!("[conn] Incoming TCP from {}", peer_addr_str);
                        let this_clone = this.clone();
                        tokio::spawn(async move {
                            this_clone
                                .accept_connection(stream, peer_addr_str)
                                .await;
                        });
                    }
                    Err(e) => warn!("[conn] Accept error: {}", e),
                }
            }
        });

        Ok(())
    }

    async fn accept_connection(
        self: &Arc<Self>,
        mut stream: TcpStream,
        addr: String,
    ) {
        // Read hello frame first
        let peer_id = match read_frame(&mut stream).await {
            Ok(Some(Frame::Hello { id })) => id,
            Ok(Some(_)) => {
                warn!("[conn] Expected hello from {}, got other frame", addr);
                return;
            }
            Ok(None) => {
                info!("[conn] Connection closed during hello from {}", addr);
                return;
            }
            Err(e) => {
                warn!("[conn] Error reading hello from {}: {}", addr, e);
                return;
            }
        };

        info!("[conn] Peer {} identified as {}", addr, peer_id);

        // Send our hello
        let my_id = get_my_id().await;
        if let Err(e) = write_frame(&mut stream, &Frame::Hello { id: my_id }).await {
            warn!("[conn] Failed to send hello to {}: {}", peer_id, e);
            return;
        }

        // Split stream for concurrent read/write
        let (reader, writer) = stream.into_split();
        let conn = Arc::new(PeerConnection {
            peer_id: peer_id.clone(),
            addr: addr.clone(),
            writer: Mutex::new(writer),
        });

        {
            let mut conns = self.connections.lock().await;
            conns.insert(peer_id.clone(), conn);
        }

        // Notify other peers about the new connection
        self.broadcast(&Frame::System {
            content: format!("peer {} is online", peer_id),
        })
        .await;

        // Spawn read loop
        self.spawn_read_loop(peer_id.clone(), reader);
    }

    pub async fn connect_to_peer(
        self: &Arc<Self>,
        peer_id: &str,
        peer_addr: &str,
    ) -> Result<(), String> {
        let addr = format!("{}:{}", peer_addr, LISTEN_PORT);
        info!("[conn] Connecting to {} at {}", peer_id, addr);
        let mut stream =
            TcpStream::connect(&addr)
                .await
                .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

        // Send our hello
        let my_id = get_my_id().await;
        write_frame(&mut stream, &Frame::Hello { id: my_id }).await?;

        // Read peer hello
        let peer_id = match read_frame(&mut stream).await {
            Ok(Some(Frame::Hello { id })) => id,
            Ok(Some(other)) => {
                return Err(format!("Expected hello, got: {:?}", other));
            }
            Ok(None) => return Err("Connection closed during hello handshake".to_string()),
            Err(e) => return Err(e),
        };

        info!("[conn] Connected to peer {}", peer_id);

        let (reader, writer) = stream.into_split();
        let conn = Arc::new(PeerConnection {
            peer_id: peer_id.clone(),
            addr: addr.clone(),
            writer: Mutex::new(writer),
        });

        {
            let mut conns = self.connections.lock().await;
            conns.insert(peer_id.clone(), conn);
        }

        // Notify other peers about the new connection
        self.broadcast(&Frame::System {
            content: format!("peer {} is online", peer_id),
        })
        .await;

        self.spawn_read_loop(peer_id.clone(), reader);

        Ok(())
    }

    fn spawn_read_loop(self: &Arc<Self>, peer_id: String, reader: OwnedReadHalf) {
        let this = self.clone();
        let frame_tx = self.frame_tx.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut buf_reader = tokio::io::BufReader::new(reader);
            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match read_frame(&mut buf_reader).await {
                    Ok(Some(frame)) => {
                        match &frame {
                            Frame::Ping => {
                                // Respond with pong — grab the Arc out so the
                                // connections lock is not held across await.
                                let conn = {
                                    let conns = this.connections.lock().await;
                                    conns.get(&peer_id).cloned()
                                };
                                if let Some(conn) = conn {
                                    let mut writer = conn.writer.lock().await;
                                    let _ = write_frame(&mut *writer, &Frame::Pong).await;
                                }
                            }
                            Frame::Pong => {
                                // Heartbeat response — nothing to do
                            }
                            _ => {
                                let _ = frame_tx.send(IncomingFrame {
                                    peer_id: peer_id.clone(),
                                    frame,
                                });
                            }
                        }
                    }
                    Ok(None) => {
                        info!("[conn] {} disconnected", peer_id);
                        break;
                    }
                    Err(e) => {
                        warn!("[conn] Read error from {}: {}", peer_id, e);
                        break;
                    }
                }
            }

            // Notify remaining peers about disconnect (broadcast before removing)
            this.broadcast(&Frame::System {
                content: format!("peer {} is offline", peer_id),
            })
            .await;

            // Cleanup connection on disconnect
            let mut conns = this.connections.lock().await;
            conns.remove(&peer_id);
            // Notify via internal channel
            let _ = frame_tx.send(IncomingFrame {
                peer_id: format!("__disconnected:{}", peer_id),
                frame: Frame::System {
                    content: format!("peer {} disconnected", peer_id),
                },
            });
        });
    }

    pub async fn send(&self, peer_id: &str, frame: &Frame) -> Result<(), String> {
        let conn = {
            let conns = self.connections.lock().await;
            conns.get(peer_id).cloned()
        };
        match conn {
            Some(conn) => {
                let mut writer = conn.writer.lock().await;
                write_frame(&mut *writer, frame).await
            }
            None => Err(format!("No connection to peer {}", peer_id)),
        }
    }

    pub async fn broadcast(&self, frame: &Frame) {
        // Collect peer Arcs so the connections lock is dropped before I/O.
        let peers = {
            let conns = self.connections.lock().await;
            collect_peers(&conns)
        };

        // Send to all peers concurrently for better throughput
        let handles: Vec<_> = peers
            .into_iter()
            .map(|conn| {
                let frame = frame.clone();
                tokio::spawn(async move {
                    let mut writer = conn.writer.lock().await;
                    if let Err(e) = write_frame(&mut *writer, &frame).await {
                        warn!("[conn] Failed to broadcast to {}: {}", conn.peer_id, e);
                    }
                })
            })
            .collect();

        for handle in handles {
            let _ = handle.await;
        }
    }

    pub async fn connected_peers(&self) -> Vec<String> {
        let conns = self.connections.lock().await;
        conns.keys().cloned().collect()
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

// Temporary — will be replaced by discovery module's identity
use std::sync::LazyLock;
use std::sync::Mutex as StdMutex;
static MY_ID: LazyLock<StdMutex<String>> = LazyLock::new(|| StdMutex::new(String::new()));

pub(crate) async fn set_my_id(id: String) {
    let mut my_id = MY_ID.lock().unwrap();
    *my_id = id;
}

async fn get_my_id() -> String {
    MY_ID.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// Creates a connected TCP loopback pair and returns the write half plus
    /// the server-side stream (which the caller must keep alive so the writer
    /// does not get connection-reset errors).
    async fn make_writer() -> (OwnedWriteHalf, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (stream_result, accept_result) = tokio::join!(
            TcpStream::connect(addr),
            listener.accept(),
        );
        let stream = stream_result.unwrap();
        let (server, _addr) = accept_result.unwrap();
        let (_reader, writer) = stream.into_split();
        (writer, server)
    }

    /// Creates a [`PeerConnection`] whose writer is backed by a live TCP socket.
    /// The returned [`TcpStream`] must stay alive for the writer to remain valid.
    async fn make_peer(id: &str) -> (Arc<PeerConnection>, TcpStream) {
        let (writer, server) = make_writer().await;
        let conn = Arc::new(PeerConnection {
            peer_id: id.to_string(),
            addr: format!("127.0.0.1:{}", server.local_addr().unwrap().port()),
            writer: Mutex::new(writer),
        });
        (conn, server)
    }

    // ------------------------------------------------------------------
    // ConnectionManager construction / lifecycle
    // ------------------------------------------------------------------

    #[test]
    fn test_new_manager_starts_running() {
        let cm = ConnectionManager::new();
        assert!(cm.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_stop_sets_running_false() {
        let cm = ConnectionManager::new();
        cm.stop();
        assert!(!cm.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_subscribe_receiver_is_open() {
        let cm = ConnectionManager::new();
        let mut rx = cm.subscribe();
        match rx.try_recv() {
            Err(broadcast::error::TryRecvError::Empty) => { /* expected */ }
            other => panic!("Expected Empty, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // ConnectionManager – state (empty manager)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_empty_manager_has_no_peers() {
        let cm = ConnectionManager::new();
        assert!(cm.connected_peers().await.is_empty());
    }

    #[tokio::test]
    async fn test_send_to_nonexistent_peer_errors() {
        let cm = ConnectionManager::new();
        let err = cm.send("nobody", &Frame::Ping).await.unwrap_err();
        assert!(err.contains("No connection"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn test_broadcast_empty_manager_does_not_panic() {
        let cm = ConnectionManager::new();
        cm.broadcast(&Frame::System { content: "test".into() }).await;
        // reachable – no panic
    }

    // ------------------------------------------------------------------
    // ConnectionManager – injected connections
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_inject_single_connection() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn, _keep) = make_peer("peer-a").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("peer-a".to_string(), conn);
        }

        let peers = cm.connected_peers().await;
        assert_eq!(peers.len(), 1);
        assert!(peers.contains(&"peer-a".to_string()));
    }

    #[tokio::test]
    async fn test_send_to_injected_peer_succeeds() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn, _keep) = make_peer("peer-a").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("peer-a".to_string(), conn);
        }

        // Should not error – the writer is connected to a live socket.
        cm.send("peer-a", &Frame::Ping).await.unwrap();
    }

    #[tokio::test]
    async fn test_inject_multiple_connections() {
        let cm = Arc::new(ConnectionManager::new());
        let mut keep = Vec::new();

        for id in &["alice", "bob", "carol"] {
            let (conn, server) = make_peer(id).await;
            keep.push(server);
            let mut conns = cm.connections.lock().await;
            conns.insert(id.to_string(), conn);
        }

        let peers = cm.connected_peers().await;
        assert_eq!(peers.len(), 3);
        for id in &["alice", "bob", "carol"] {
            assert!(peers.contains(&id.to_string()), "missing {id}");
        }
    }

    #[tokio::test]
    async fn test_duplicate_peer_id_overwrites_previous() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn_a, _keep_a) = make_peer("peer-x").await;
        let (conn_b, _keep_b) = make_peer("peer-x").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("peer-x".to_string(), conn_a);
            conns.insert("peer-x".to_string(), conn_b); // overwrite
        }

        let peers = cm.connected_peers().await;
        assert_eq!(peers.len(), 1, "overwritten peer still counted twice");
        assert!(peers.contains(&"peer-x".to_string()));
    }

    #[tokio::test]
    async fn test_send_to_specific_peer_among_many() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn_a, _keep_a) = make_peer("peer-a").await;
        let (conn_b, _keep_b) = make_peer("peer-b").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("peer-a".to_string(), conn_a);
            conns.insert("peer-b".to_string(), conn_b);
        }

        cm.send("peer-a", &Frame::Ping).await.unwrap();
        cm.send("peer-b", &Frame::System { content: "hi".into() })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_send_to_nonexistent_among_valid_peers() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn, _keep) = make_peer("peer-a").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("peer-a".to_string(), conn);
        }

        let err = cm.send("peer-b", &Frame::Ping).await.unwrap_err();
        assert!(err.contains("No connection"));
    }

    #[tokio::test]
    async fn test_broadcast_to_multiple_peers_succeeds() {
        let cm = Arc::new(ConnectionManager::new());
        let mut keep = Vec::new();

        for id in &["alice", "bob"] {
            let (conn, server) = make_peer(id).await;
            keep.push(server);
            let mut conns = cm.connections.lock().await;
            conns.insert(id.to_string(), conn);
        }

        // Must not panic even though peers exist.
        cm.broadcast(&Frame::System { content: "hi everyone".into() })
            .await;
    }

    // ------------------------------------------------------------------
    // ConnectionManager – removal / cleanup
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_connection_removal() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn, _keep) = make_peer("peer-a").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("peer-a".to_string(), conn);
        }
        assert_eq!(cm.connected_peers().await.len(), 1);

        // Remove the peer.
        {
            let mut conns = cm.connections.lock().await;
            conns.remove("peer-a");
        }
        assert!(cm.connected_peers().await.is_empty());

        // Sending to the removed peer must now fail.
        let err = cm.send("peer-a", &Frame::Ping).await.unwrap_err();
        assert!(err.contains("No connection"));
    }

    #[tokio::test]
    async fn test_remove_nonexistent_peer_is_noop() {
        let cm = Arc::new(ConnectionManager::new());
        {
            let mut conns = cm.connections.lock().await;
            conns.remove("nobody"); // must not panic
        }
        assert!(cm.connected_peers().await.is_empty());
    }

    #[tokio::test]
    async fn test_stop_does_not_clear_connections() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn, _keep) = make_peer("peer-a").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("peer-a".to_string(), conn);
        }

        cm.stop();
        assert!(!cm.running.load(Ordering::SeqCst));

        // Connections are still accessible after stop.
        assert_eq!(cm.connected_peers().await.len(), 1);
    }

    // ------------------------------------------------------------------
    // collect_peers (standalone helper)
    // ------------------------------------------------------------------

    #[test]
    fn test_collect_peers_empty_map() {
        let map: HashMap<String, Arc<PeerConnection>> = HashMap::new();
        let result = collect_peers(&map);
        assert!(result.is_empty());
    }

    // ------------------------------------------------------------------
    // identity helpers (set_my_id / get_my_id)
    //
    // These use a global std::sync::Mutex so that tests running on
    // separate tokio threads do not clobber each other's ID.
    // ------------------------------------------------------------------

    static ID_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn test_set_and_get_my_id() {
        let _guard = ID_TEST_LOCK.lock().unwrap();
        set_my_id("test-host-abc".to_string()).await;
        assert_eq!(get_my_id().await, "test-host-abc");
    }

    #[tokio::test]
    async fn test_set_my_id_overwrites_previous() {
        let _guard = ID_TEST_LOCK.lock().unwrap();
        set_my_id("first".to_string()).await;
        set_my_id("second".to_string()).await;
        assert_eq!(get_my_id().await, "second");
    }

    // ------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_empty_peer_id_accepted() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn, _keep) = make_peer("").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert(String::new(), conn);
        }

        let peers = cm.connected_peers().await;
        assert_eq!(peers.len(), 1);
        assert!(peers.contains(&String::new()));

        cm.send("", &Frame::Ping).await.unwrap();
    }

    #[tokio::test]
    async fn test_special_char_peer_id() {
        let cm = Arc::new(ConnectionManager::new());
        let special = "peer-123_abc@host!";
        let (conn, _keep) = make_peer(special).await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert(special.to_string(), conn);
        }

        let peers = cm.connected_peers().await;
        assert!(peers.contains(&special.to_string()));

        cm.send(special, &Frame::Ping).await.unwrap();
    }

    #[tokio::test]
    async fn test_connected_peers_snapshot_independent_of_concurrent_modification() {
        // Verify that connected_peers (and internally collect_peers) takes a
        // snapshot: adding after the call is not reflected in the returned vec.
        let cm = Arc::new(ConnectionManager::new());

        let peers_before = cm.connected_peers().await;
        assert!(peers_before.is_empty());

        let (conn, _keep) = make_peer("late-peer").await;
        {
            let mut conns = cm.connections.lock().await;
            conns.insert("late-peer".to_string(), conn);
        }

        let peers_after = cm.connected_peers().await;
        assert_eq!(peers_after.len(), 1);
    }

    #[tokio::test]
    async fn test_broadcast_snapshot_isolated_from_removal() {
        // broadcast collects peers first, then writes.  If a peer is removed
        // between the snapshot and the I/O the broadcast still uses the
        // snapshot (harmlessly writing to a dropped writer handle).
        let cm = Arc::new(ConnectionManager::new());
        let (conn, _keep) = make_peer("ephemeral").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("ephemeral".to_string(), conn);
        }

        // Remove the peer immediately before broadcast so the snapshot is
        // taken while the peer still exists.
        let peers = {
            let conns = cm.connections.lock().await;
            collect_peers(&conns)
        };
        {
            let mut conns = cm.connections.lock().await;
            conns.remove("ephemeral");
        }

        // The snapshot still holds an Arc, so writing to it should be safe
        // (the writer may be broken since the server end was dropped, but the
        // broadcast only logs the error – it never panics).
        for conn in &peers {
            // This is exactly what broadcast does internally, minus the
            // warn! — we just verify it doesn't panic.
            let mut writer = conn.writer.lock().await;
            let _ = write_frame(&mut *writer, &Frame::Ping).await;
        }
    }

    // ------------------------------------------------------------------
    // Broadcast — content verification
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_broadcast() {
        let cm = Arc::new(ConnectionManager::new());
        let (conn_a, mut server_a) = make_peer("alice").await;
        let (conn_b, mut server_b) = make_peer("bob").await;

        {
            let mut conns = cm.connections.lock().await;
            conns.insert("alice".to_string(), conn_a);
            conns.insert("bob".to_string(), conn_b);
        }

        // Broadcast a system frame to all peers
        cm.broadcast(&Frame::System {
            content: "hello everyone".into(),
        })
        .await;

        // Read frames back from each server-side socket to verify delivery
        let frame_a = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            read_frame(&mut server_a),
        )
        .await
        .expect("timeout reading from alice's server socket")
        .expect("read_frame error for alice")
        .expect("alice should have received a frame (not EOF)");

        let frame_b = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            read_frame(&mut server_b),
        )
        .await
        .expect("timeout reading from bob's server socket")
        .expect("read_frame error for bob")
        .expect("bob should have received a frame (not EOF)");

        // Verify both peers received the correct frame
        match &frame_a {
            Frame::System { content } => assert_eq!(content, "hello everyone"),
            other => panic!("Expected System frame for alice, got {:?}", other),
        }
        match &frame_b {
            Frame::System { content } => assert_eq!(content, "hello everyone"),
            other => panic!("Expected System frame for bob, got {:?}", other),
        }
    }
}
