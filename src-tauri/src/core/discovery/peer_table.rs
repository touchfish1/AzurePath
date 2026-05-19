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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::discovery::PeerInfo;

    fn make_peer(id: &str, ip: &str, hostname: &str, last_seen: &str) -> PeerInfo {
        PeerInfo {
            id: id.to_string(),
            hostname: hostname.to_string(),
            ip: ip.to_string(),
            os: "linux".to_string(),
            listen_port: 42070,
            last_seen: last_seen.to_string(),
            status: "online".to_string(),
        }
    }

    #[tokio::test]
    async fn test_upsert_new_peer() {
        let table = PeerTable::new();
        let peer = make_peer("id1", "192.168.1.10", "desktop", "2025-01-01T00:00:00+00:00");

        let is_new = table.upsert(peer).await;
        assert!(is_new);

        let list = table.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].ip, "192.168.1.10");
    }

    #[tokio::test]
    async fn test_upsert_existing_peer_overwrites() {
        let table = PeerTable::new();
        let peer1 = make_peer("id1", "192.168.1.10", "old-host", "2025-01-01T00:00:00+00:00");
        assert!(table.upsert(peer1).await);

        let peer2 = make_peer("id1", "192.168.1.20", "new-host", "2025-01-01T00:00:30+00:00");
        let is_new = table.upsert(peer2).await;
        assert!(!is_new, "same id should not be considered new");

        let list = table.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].ip, "192.168.1.20");
        assert_eq!(list[0].hostname, "new-host");
    }

    #[tokio::test]
    async fn test_mark_offline() {
        let table = PeerTable::new();
        let peer = make_peer("id1", "192.168.1.10", "host", "2025-01-01T00:00:00+00:00");
        table.upsert(peer).await;

        assert!(table.mark_offline("id1").await);
        assert!(!table.mark_offline("nonexistent").await);

        let list = table.list().await;
        assert_eq!(list[0].status, "offline");
    }

    #[tokio::test]
    async fn test_online_peers_filter() {
        let table = PeerTable::new();

        let online = make_peer("id1", "10.0.0.1", "host1", "2025-01-01T00:00:00+00:00");
        table.upsert(online).await;

        let offline = make_peer("id2", "10.0.0.2", "host2", "2025-01-01T00:00:00+00:00");
        table.upsert(offline).await;
        table.mark_offline("id2").await;

        let online_list = table.online_peers().await;
        assert_eq!(online_list.len(), 1);
        assert_eq!(online_list[0].id, "id1");
    }

    #[tokio::test]
    async fn test_check_stale_marks_offline() {
        let table = PeerTable::new();
        let old_time = "2020-01-01T00:00:00+00:00"; // way in the past
        let peer = make_peer("id1", "10.0.0.1", "host1", old_time);
        table.upsert(peer).await;

        let stale = table.check_stale(10).await; // 10 second timeout
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0], "id1");

        // After check_stale, the peer should be marked offline
        let list = table.list().await;
        assert_eq!(list[0].status, "offline");
    }

    #[tokio::test]
    async fn test_check_stale_skips_recent() {
        let table = PeerTable::new();
        let recent = chrono::Utc::now().to_rfc3339();
        let peer = make_peer("id1", "10.0.0.1", "host1", &recent);
        table.upsert(peer).await;

        let stale = table.check_stale(60).await; // 60 second timeout (longer than any test duration)
        assert!(stale.is_empty(), "recent peer should not be stale");

        let list = table.list().await;
        assert_eq!(list[0].status, "online");
    }

    #[tokio::test]
    async fn test_get_peer() {
        let table = PeerTable::new();
        let peer = make_peer("id1", "10.0.0.1", "host1", "2025-01-01T00:00:00+00:00");
        table.upsert(peer).await;

        let found = table.get("id1").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().hostname, "host1");

        assert!(table.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_list_sorted_by_last_seen_desc() {
        let table = PeerTable::new();

        let old = make_peer("id1", "10.0.0.1", "old", "2024-01-01T00:00:00+00:00");
        let recent = make_peer("id2", "10.0.0.2", "recent", "2025-01-01T00:00:00+00:00");

        table.upsert(old).await;
        table.upsert(recent).await;

        let list = table.list().await;
        assert_eq!(list.len(), 2);
        // Most recent should be first
        assert_eq!(list[0].id, "id2");
        assert_eq!(list[1].id, "id1");
    }

    #[tokio::test]
    async fn test_peer_ip_override_from_src() {
        // Simulate what the discovery listener does now: deserialize a PeerInfo
        // that was broadcast with ip="0.0.0.0", then override the IP from the
        // UDP source address before upserting.
        let mut peer = make_peer("id1", "0.0.0.0", "host1", "2025-01-01T00:00:00+00:00");

        // This is the fix: override IP with the actual source address
        peer.ip = "192.168.1.50".to_string();

        let table = PeerTable::new();
        table.upsert(peer).await;

        let found = table.get("id1").await.unwrap();
        assert_eq!(found.ip, "192.168.1.50", "IP should be the real source address, not 0.0.0.0");
        assert_ne!(found.ip, "0.0.0.0", "IP must not be the placeholder");
    }
}
