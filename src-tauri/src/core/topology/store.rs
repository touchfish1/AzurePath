//! SQLite store for topology snapshots.

use rusqlite::{params, Connection};
use std::sync::Mutex;

use crate::types::topology::{SnapshotDetail, SnapshotDiff, TopologyLink, TopologyNode, TopologySnapshot};

pub struct TopologyStore {
    conn: Mutex<Connection>,
}

impl TopologyStore {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| e.to_string())?;
        let store = Self { conn: Mutex::new(conn) };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS topology_snapshots (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                layout_algorithm TEXT NOT NULL DEFAULT 'forceDirected',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS topology_snapshot_nodes (
                snapshot_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                ip TEXT NOT NULL,
                hostname TEXT,
                device_type TEXT,
                vendor TEXT,
                model TEXT,
                os TEXT,
                status TEXT,
                x REAL NOT NULL DEFAULT 0,
                y REAL NOT NULL DEFAULT 0,
                group_id TEXT,
                PRIMARY KEY (snapshot_id, node_id)
            );
            CREATE TABLE IF NOT EXISTS topology_snapshot_links (
                snapshot_id TEXT NOT NULL,
                link_id TEXT NOT NULL,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                link_type TEXT DEFAULT 'wired',
                latency_ms REAL,
                PRIMARY KEY (snapshot_id, link_id)
            );"
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_snapshot(&self, name: &str, layout_algorithm: &str, nodes: &[TopologyNode], links: &[TopologyLink]) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO topology_snapshots (id, name, layout_algorithm, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, layout_algorithm, now],
        ).map_err(|e| e.to_string())?;

        for node in nodes {
            conn.execute(
                "INSERT INTO topology_snapshot_nodes (snapshot_id, node_id, ip, hostname, device_type, vendor, model, os, status, x, y, group_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![id, node.id, node.ip, node.hostname, node.device_type, node.vendor, node.model, node.os, node.status, node.x, node.y, node.group_id],
            ).map_err(|e| e.to_string())?;
        }

        for link in links {
            conn.execute(
                "INSERT INTO topology_snapshot_links (snapshot_id, link_id, source_id, target_id, link_type, latency_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, link.id, link.source_id, link.target_id, link.link_type, link.latency_ms],
            ).map_err(|e| e.to_string())?;
        }

        Ok(id)
    }

    pub fn list_snapshots(&self) -> Result<Vec<TopologySnapshot>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, name, created_at FROM topology_snapshots ORDER BY created_at DESC"
        ).map_err(|e| e.to_string())?;

        let snapshots = stmt.query_map([], |row| {
            Ok(TopologySnapshot {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(snapshots)
    }

    pub fn load_snapshot(&self, id: &str) -> Result<SnapshotDetail, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut snap_stmt = conn.prepare(
            "SELECT id, name, layout_algorithm, created_at FROM topology_snapshots WHERE id = ?1"
        ).map_err(|e| e.to_string())?;

        let detail = snap_stmt.query_row(params![id], |row| {
            let snap_id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let layout: String = row.get(2)?;
            let created: String = row.get(3)?;
            Ok((snap_id, name, layout, created))
        }).map_err(|e| e.to_string())?;

        let mut node_stmt = conn.prepare(
            "SELECT node_id, ip, hostname, device_type, vendor, model, os, status, x, y, group_id \
             FROM topology_snapshot_nodes WHERE snapshot_id = ?1"
        ).map_err(|e| e.to_string())?;

        let nodes: Vec<TopologyNode> = node_stmt.query_map(params![id], |row| {
            let group: Option<String> = row.get(10)?;
            Ok(TopologyNode {
                id: row.get(0)?,
                ip: row.get(1)?,
                hostname: row.get(2)?,
                device_type: row.get(3)?,
                vendor: row.get(4)?,
                model: row.get(5)?,
                os: row.get(6)?,
                status: row.get(7)?,
                x: row.get(8)?,
                y: row.get(9)?,
                group_id: group,
                cpu_usage: None,
                memory_usage: None,
                mac: String::new(),
                interfaces: vec![],
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        let mut link_stmt = conn.prepare(
            "SELECT link_id, source_id, target_id, link_type, latency_ms \
             FROM topology_snapshot_links WHERE snapshot_id = ?1"
        ).map_err(|e| e.to_string())?;

        let links: Vec<TopologyLink> = link_stmt.query_map(params![id], |row| {
            Ok(TopologyLink {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                link_type: row.get(3)?,
                latency_ms: row.get(4)?,
                speed: None,
                bandwidth_usage: None,
                source_iface: None,
                target_iface: None,
            })
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(SnapshotDetail {
            id: detail.0,
            name: detail.1,
            layout_algorithm: detail.2,
            created_at: detail.3,
            nodes,
            links,
        })
    }

    pub fn delete_snapshot(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM topology_snapshot_links WHERE snapshot_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM topology_snapshot_nodes WHERE snapshot_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM topology_snapshots WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn compare_snapshots(&self, id_a: &str, id_b: &str) -> Result<SnapshotDiff, String> {
        let a = self.load_snapshot(id_a)?;
        let b = self.load_snapshot(id_b)?;

        let a_ips: std::collections::HashSet<String> = a.nodes.iter().map(|n| n.ip.clone()).collect();
        let b_ips: std::collections::HashSet<String> = b.nodes.iter().map(|n| n.ip.clone()).collect();

        let added: Vec<String> = b_ips.difference(&a_ips).cloned().collect();
        let removed: Vec<String> = a_ips.difference(&b_ips).cloned().collect();

        let mut changed = Vec::new();
        for node_a in &a.nodes {
            if let Some(node_b) = b.nodes.iter().find(|n| n.ip == node_a.ip) {
                if node_a.status != node_b.status || node_a.device_type != node_b.device_type {
                    changed.push(node_a.ip.clone());
                }
            }
        }

        Ok(SnapshotDiff { added_nodes: added, removed_nodes: removed, changed_nodes: changed })
    }
}
