use crate::types::discovery::PeerInfo;
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct PeerTable {
    peers: Mutex<HashMap<String, PeerInfo>>,
}

impl PeerTable {
    pub fn new() -> Self {
        Self {
            peers: Mutex::new(HashMap::new()),
        }
    }

    /// Upsert a peer record. Returns true if the peer was newly added.
    pub async fn upsert(&self, info: PeerInfo) -> bool {
        let mut peers = self.peers.lock().await;
        let is_new = !peers.contains_key(&info.id);
        peers.insert(info.id.clone(), info);
        is_new
    }

    /// Mark peer as offline. Returns true if the peer existed.
    pub async fn mark_offline(&self, id: &str) -> bool {
        let mut peers = self.peers.lock().await;
        if let Some(peer) = peers.get_mut(id) {
            peer.status = "offline".to_string();
            true
        } else {
            false
        }
    }

    /// Get a copy of the peer list.
    pub async fn list(&self) -> Vec<PeerInfo> {
        let peers = self.peers.lock().await;
        let mut list: Vec<PeerInfo> = peers.values().cloned().collect();
        list.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
        list
    }

    /// Get online peers only.
    pub async fn online_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.lock().await;
        peers
            .values()
            .filter(|p| p.status == "online")
            .cloned()
            .collect()
    }

    /// Remove stale peers (last_seen > timeout_secs ago) and mark them offline.
    /// Returns the list of peers that transitioned to offline.
    pub async fn check_stale(&self, timeout_secs: u64) -> Vec<String> {
        let mut peers = self.peers.lock().await;
        let now = chrono::Utc::now();
        let mut stale = Vec::new();

        for (id, peer) in peers.iter_mut() {
            if peer.status != "online" {
                continue;
            }
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&peer.last_seen) {
                let last_utc = last.with_timezone(&chrono::Utc);
                let elapsed = (now - last_utc).num_seconds() as u64;
                if elapsed > timeout_secs {
                    peer.status = "offline".to_string();
                    stale.push(id.clone());
                }
            }
        }
        stale
    }

    /// Get a single peer by ID.
    pub async fn get(&self, id: &str) -> Option<PeerInfo> {
        let peers = self.peers.lock().await;
        peers.get(id).cloned()
    }
}
