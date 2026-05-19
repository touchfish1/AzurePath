mod peer_table;

use crate::types::discovery::PeerInfo;
use peer_table::PeerTable;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{info, warn};

pub const DISCOVERY_PORT: u16 = 42069;
pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;
pub const STALE_TIMEOUT_SECS: u64 = 30;

static MY_ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static MY_HOSTNAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Initialize and retrieve the local peer identity.
/// Format: `hostname-{random_hex(4)}`
pub async fn init_identity() -> String {
    MY_ID
        .get_or_init(|| {
            let hostname = hostname();
            let suffix = random_hex(4);
            let id = format!("{}-{}", hostname, suffix);
            let _ = MY_HOSTNAME.set(hostname);
            id
        })
        .clone()
}

pub async fn my_id() -> String {
    MY_ID
        .get()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}

pub async fn my_hostname() -> String {
    MY_HOSTNAME
        .get()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
        .to_lowercase()
}

fn random_hex(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len).map(|_| format!("{:x}", rng.gen_range(0..16))).collect()
}

pub struct DiscoveryService {
    peer_table: Arc<PeerTable>,
    running: Arc<AtomicBool>,
}

impl DiscoveryService {
    pub fn new() -> Self {
        Self {
            peer_table: Arc::new(PeerTable::new()),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn peer_table(&self) -> &Arc<PeerTable> {
        &self.peer_table
    }

    /// Start the discovery service: bind UDP listener + heartbeat broadcaster.
    pub async fn start(self: &Arc<Self>) -> Result<(), String> {
        let my_id = init_identity().await;
        let my_hostname = my_hostname().await;

        info!("[discovery] Starting with identity: {}", my_id);

        // Set identity for the connection module
        crate::core::connection::set_my_id(my_id.clone()).await;

        // Bind UDP socket
        let bind_addr = format!("0.0.0.0:{}", DISCOVERY_PORT);
        let socket = UdpSocket::bind(&bind_addr)
            .await
            .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

        socket
            .set_broadcast(true)
            .map_err(|e| format!("Failed to set broadcast: {}", e))?;

        info!("[discovery] UDP listener on {}", bind_addr);

        // Enable broadcast socket for sending
        let broadcast_socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Failed to bind broadcast socket: {}", e))?;
        broadcast_socket
            .set_broadcast(true)
            .map_err(|e| format!("Failed to set broadcast on sender: {}", e))?;

        let this = self.clone();

        // 1. Spawn UDP listener task
        let listener_running = this.running.clone();
        let peer_table = this.peer_table.clone();
        let listener_my_id = my_id.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                if !listener_running.load(Ordering::SeqCst) {
                    break;
                }

                match socket.recv_from(&mut buf).await {
                    Ok((n, src_addr)) => {
                        let data = &buf[..n];
                        if let Ok(mut peer) = serde_json::from_slice::<PeerInfo>(data) {
                            // Skip self
                            if peer.id == listener_my_id {
                                continue;
                            }
                            // Override the IP with the actual source address from UDP.
                            // The sender always sets ip to "0.0.0.0" expecting the
                            // receiver to fill in the real value.
                            peer.ip = src_addr.ip().to_string();
                            peer.listen_port = crate::core::connection::LISTEN_PORT;
                            let hostname = peer.hostname.clone();
                            let ip = peer.ip.clone();
                            let is_new = peer_table.upsert(peer).await;
                            if is_new {
                                info!(
                                    "[discovery] New peer discovered: {} ({})",
                                    hostname, ip
                                );
                            }
                        } else {
                            warn!(
                                "[discovery] Invalid broadcast from {}: {}",
                                src_addr,
                                String::from_utf8_lossy(data)
                            );
                        }
                    }
                    Err(e) => {
                        if listener_running.load(Ordering::SeqCst) {
                            warn!("[discovery] Recv error: {}", e);
                        }
                        break;
                    }
                }
            }
        });

        // 2. Spawn heartbeat broadcaster
        let heartbeat_running = this.running.clone();
        // Invariant fields — computed once outside the loop
        let os = std::env::consts::OS.to_string();
        let listen_port = crate::core::connection::LISTEN_PORT;
        tokio::spawn(async move {
            loop {
                if !heartbeat_running.load(Ordering::SeqCst) {
                    break;
                }

                // Build peer info for broadcast
                let peer_info = PeerInfo {
                    id: my_id.clone(),
                    hostname: my_hostname.clone(),
                    ip: "0.0.0.0".to_string(), // receiver fills from source addr
                    os: os.clone(),
                    listen_port,
                    last_seen: chrono::Utc::now().to_rfc3339(),
                    status: "online".to_string(),
                };

                if let Ok(data) = serde_json::to_vec(&peer_info) {
                    let broadcast_addr = format!("255.255.255.255:{}", DISCOVERY_PORT);
                    if let Err(e) = broadcast_socket.send_to(&data, &broadcast_addr).await {
                        warn!("[discovery] Broadcast error: {}", e);
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS))
                    .await;
            }
        });

        // 3. Spawn stale peer checker
        let stale_running = this.running.clone();
        let stale_table = this.peer_table.clone();
        tokio::spawn(async move {
            loop {
                if !stale_running.load(Ordering::SeqCst) {
                    break;
                }

                let stale = stale_table.check_stale(STALE_TIMEOUT_SECS).await;
                for id in stale {
                    info!("[discovery] Peer {} went offline (timeout)", id);
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS))
                    .await;
            }
        });

        Ok(())
    }

    pub async fn get_peer(&self, id: &str) -> Option<PeerInfo> {
        self.peer_table.get(id).await
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::discovery::PeerInfo;

    // ── hostname() ────────────────────────────────────────────────────────

    #[test]
    fn test_hostname_not_empty() {
        let name = hostname();
        assert!(!name.is_empty(), "hostname() should return a non-empty string");
    }

    #[test]
    fn test_hostname_is_lowercased() {
        let name = hostname();
        assert_eq!(name, name.to_lowercase(), "hostname() must be lowercased");
    }

    #[test]
    fn test_hostname_does_not_contain_whitespace() {
        let name = hostname();
        assert!(!name.contains(char::is_whitespace), "hostname() should not contain whitespace, got: '{}'", name);
    }

    // ── random_hex() ──────────────────────────────────────────────────────

    #[test]
    fn test_random_hex_length() {
        assert_eq!(random_hex(0).len(), 0);
        assert_eq!(random_hex(1).len(), 1);
        assert_eq!(random_hex(4).len(), 4);
        assert_eq!(random_hex(8).len(), 8);
        assert_eq!(random_hex(16).len(), 16);
    }

    #[test]
    fn test_random_hex_uses_only_hex_digits() {
        let hex = random_hex(100);
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "random_hex produced non-hex characters: '{}'",
            hex
        );
    }

    #[test]
    fn test_random_hex_produces_different_values() {
        let a = random_hex(8);
        let b = random_hex(8);
        assert_ne!(a, b, "random_hex should produce varying results");
    }

    #[test]
    fn test_random_hex_zero_length() {
        assert_eq!(random_hex(0), String::new());
    }

    // ── DiscoveryService construction / stop ──────────────────────────────

    #[test]
    fn test_discovery_service_new_running_is_true() {
        let service = DiscoveryService::new();
        assert!(service.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_discovery_service_stop_sets_running_false() {
        let service = DiscoveryService::new();
        assert!(service.running.load(Ordering::SeqCst));
        service.stop();
        assert!(!service.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_discovery_service_stop_is_idempotent() {
        let service = DiscoveryService::new();
        service.stop();
        service.stop(); // second call must not panic
        assert!(!service.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_discovery_service_peer_table_initial_empty() {
        let service = DiscoveryService::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let peers = rt.block_on(service.peer_table().list());
        assert!(peers.is_empty());
    }

    #[test]
    fn test_discovery_service_get_peer_nonexistent() {
        let service = DiscoveryService::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let peer = rt.block_on(service.get_peer("nonexistent"));
        assert!(peer.is_none());
    }

    // ── Identity lifecycle ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_init_identity_format() {
        let id = init_identity().await;
        assert!(!id.is_empty(), "identity must not be empty");

        // Format: <hostname>-<4-hex-chars>
        let parts: Vec<&str> = id.split('-').collect();
        assert!(parts.len() >= 2, "identity must contain a dash, got: '{}'", id);

        let suffix = parts.last().unwrap();
        assert_eq!(suffix.len(), 4, "suffix must be exactly 4 hex chars, got: '{}' from '{}'", suffix, id);
        assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()), "suffix must be hex, got: '{}'", suffix);
    }

    #[tokio::test]
    async fn test_init_identity_returns_consistent_hostname_prefix() {
        let id = init_identity().await;
        let host = hostname();
        // The id should begin with the hostname
        assert!(id.starts_with(&host), "identity '{}' should start with hostname '{}'", id, host);
    }

    #[tokio::test]
    async fn test_init_identity_idempotent() {
        let id1 = init_identity().await;
        let id2 = init_identity().await;
        assert_eq!(id1, id2, "init_identity() must return the same value on every call");
    }

    #[tokio::test]
    async fn test_my_id_after_init() {
        let id = init_identity().await;
        let my = my_id().await;
        assert_eq!(my, id, "my_id() must return the same value as init_identity()");
    }

    #[tokio::test]
    async fn test_my_hostname_after_init() {
        let _ = init_identity().await;
        let host = my_hostname().await;
        assert!(!host.is_empty(), "my_hostname() must return a non-empty string after init");
        assert_eq!(host, host.to_lowercase(), "my_hostname() must be lowercased");
    }

    // ── Peer table integration (DiscoveryService wrapper) ─────────────────

    #[tokio::test]
    async fn test_discovery_service_upsert_and_retrieve() {
        let service = DiscoveryService::new();
        let table = service.peer_table();

        let peer = PeerInfo {
            id: "integration-peer".into(),
            hostname: "integration-host".into(),
            ip: "10.0.0.99".into(),
            os: "linux".into(),
            listen_port: 42070,
            last_seen: "2025-06-01T00:00:00+00:00".into(),
            status: "online".into(),
        };

        let is_new = table.upsert(peer.clone()).await;
        assert!(is_new, "first upsert must return true");

        let found = service.get_peer("integration-peer").await;
        assert!(found.is_some(), "peer should be retrievable via get_peer");
        assert_eq!(found.as_ref().unwrap().hostname, "integration-host");
        assert_eq!(found.as_ref().unwrap().ip, "10.0.0.99");
    }

    // ── PeerInfo serialization ────────────────────────────────────────────

    #[test]
    fn test_peer_info_serialization_roundtrip() {
        let original = PeerInfo {
            id: "serde-test-peer".into(),
            hostname: "serde-host".into(),
            ip: "0.0.0.0".into(),
            os: "windows".into(),
            listen_port: 42070,
            last_seen: "2025-06-01T12:00:00+00:00".into(),
            status: "online".into(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: PeerInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "serde-test-peer");
        assert_eq!(deserialized.hostname, "serde-host");
        assert_eq!(deserialized.ip, "0.0.0.0");
        assert_eq!(deserialized.os, "windows");
        assert_eq!(deserialized.listen_port, 42070);
        assert_eq!(deserialized.last_seen, "2025-06-01T12:00:00+00:00");
        assert_eq!(deserialized.status, "online");
    }

    #[test]
    fn test_peer_info_serialization_preserves_real_ip() {
        let peer = PeerInfo {
            id: "real-ip-peer".into(),
            hostname: "laptop".into(),
            ip: "192.168.1.42".into(),
            os: "macos".into(),
            listen_port: 42070,
            last_seen: "2025-06-15T08:30:00+00:00".into(),
            status: "online".into(),
        };

        let json = serde_json::to_string(&peer).unwrap();
        let deserialized: PeerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.ip, "192.168.1.42");
    }

    #[test]
    fn test_peer_info_json_field_names() {
        // Verify that serde serialization produces the expected field names
        let peer = PeerInfo {
            id: "f1".into(),
            hostname: "h".into(),
            ip: "10.0.0.1".into(),
            os: "os".into(),
            listen_port: 42070,
            last_seen: "ts".into(),
            status: "online".into(),
        };
        let json = serde_json::to_value(&peer).unwrap();
        let map = json.as_object().unwrap();
        assert!(map.contains_key("id"));
        assert!(map.contains_key("hostname"));
        assert!(map.contains_key("ip"));
        assert!(map.contains_key("os"));
        assert!(map.contains_key("listen_port"));
        assert!(map.contains_key("last_seen"));
        assert!(map.contains_key("status"));
    }

    // ── Constants ─────────────────────────────────────────────────────────

    #[test]
    fn test_discovery_port_constant() {
        assert_eq!(DISCOVERY_PORT, 42069);
    }

    #[test]
    fn test_heartbeat_interval_constant() {
        assert_eq!(HEARTBEAT_INTERVAL_SECS, 5);
    }

    #[test]
    fn test_stale_timeout_constant() {
        assert_eq!(STALE_TIMEOUT_SECS, 30);
    }
}
