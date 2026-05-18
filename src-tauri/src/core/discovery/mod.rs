mod peer_table;

use crate::types::discovery::PeerInfo;
use peer_table::PeerTable;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;

pub const DISCOVERY_PORT: u16 = 42069;
pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;
pub const STALE_TIMEOUT_SECS: u64 = 30;

static MY_ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static MY_HOSTNAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Initialize and retrieve the local peer identity.
/// Format: `hostname-{random_hex(4)}`
pub async fn init_identity() -> String {
    if let Some(id) = MY_ID.get() {
        return id.clone();
    }

    let hostname = hostname();
    let suffix = random_hex(4);
    let id = format!("{}-{}", hostname, suffix);

    let _ = MY_ID.set(id.clone());
    let _ = MY_HOSTNAME.set(hostname);
    id
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

        println!("[discovery] Starting with identity: {}", my_id);

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

        println!("[discovery] UDP listener on {}", bind_addr);

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
                        if let Ok(peer) = serde_json::from_slice::<PeerInfo>(data) {
                            // Skip self
                            if peer.id == listener_my_id {
                                continue;
                            }
                            let hostname = peer.hostname.clone();
                            let ip = peer.ip.clone();
                            let is_new = peer_table.upsert(peer).await;
                            if is_new {
                                println!(
                                    "[discovery] New peer discovered: {} ({})",
                                    hostname, ip
                                );
                            }
                        } else {
                            eprintln!(
                                "[discovery] Invalid broadcast from {}: {}",
                                src_addr,
                                String::from_utf8_lossy(data)
                            );
                        }
                    }
                    Err(e) => {
                        if listener_running.load(Ordering::SeqCst) {
                            eprintln!("[discovery] Recv error: {}", e);
                        }
                        break;
                    }
                }
            }
        });

        // 2. Spawn heartbeat broadcaster
        let heartbeat_running = this.running.clone();
        tokio::spawn(async move {
            loop {
                if !heartbeat_running.load(Ordering::SeqCst) {
                    break;
                }

                // Build peer info for broadcast
                let os = std::env::consts::OS.to_string();
                let peer_info = PeerInfo {
                    id: my_id.clone(),
                    hostname: my_hostname.clone(),
                    ip: "0.0.0.0".to_string(), // receiver fills from source addr
                    os,
                    listen_port: crate::core::connection::LISTEN_PORT,
                    last_seen: chrono::Utc::now().to_rfc3339(),
                    status: "online".to_string(),
                };

                if let Ok(data) = serde_json::to_vec(&peer_info) {
                    let broadcast_addr = format!("255.255.255.255:{}", DISCOVERY_PORT);
                    if let Err(e) = broadcast_socket.send_to(&data, &broadcast_addr).await {
                        eprintln!("[discovery] Broadcast error: {}", e);
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
                    println!("[discovery] Peer {} went offline (timeout)", id);
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
