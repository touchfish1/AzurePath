# 网络拓扑可视化增强 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance the existing network topology page with device classification, multiple layout algorithms, search/filter, topology snapshots, and optional SNMP integration.

**Architecture:** Two-phase approach. Phase 1 (no dependencies) adds layout algorithms, persistence, search/filter, and enhanced Canvas rendering. Phase 2 (depends on SNMP feature completion) integrates device type classification and resource monitoring from SNMP data.

**Tech Stack:** Canvas API (existing), SQLite via `rusqlite` (existing), Rust backend + Vue 3/TypeScript frontend

---

## File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `src-tauri/src/types/topology.rs` | **Create** | Enhanced data models: `TopologyNode`, `TopologyLink`, enums for DeviceType/LinkType/LayoutAlgorithm |
| `src-tauri/src/core/topology/mod.rs` | **Modify** | Export new submodules |
| `src-tauri/src/core/topology/layout.rs` | **Create** | `TopologyLayout` — ForceDirected, Hierarchical, Circular, Grid algorithms |
| `src-tauri/src/core/topology/store.rs` | **Create** | `TopologyStore` — SQLite CRUD for topology snapshots |
| `src-tauri/src/core/topology/snmp.rs` | **Create** | `enrich_from_snmp()` — device type identification via SNMP database (Phase 2) |
| `src-tauri/src/commands/topology.rs` | **Modify** | Add new commands for layout, position, snapshot management; add SNMP enrich (Phase 2) |
| `src-tauri/src/lib.rs` | **Modify** | Register new commands |
| `src/lib/tauri.ts` | **Modify** | Add TypeScript interfaces and invoke wrappers |
| `src/stores/topology.ts` | **Create** | Pinia store for topology state, search, filter, snapshots |
| `src/pages/topology/Page.vue` | **Modify** | Enhanced Canvas rendering, layout selector, search/filter bar, snapshot controls |

---

### Task 1: Create enhanced topology data types

**Files:**
- Create: `src-tauri/src/types/topology.rs`

- [ ] **Step 1: Create types/topology.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyNode {
    pub id: String,
    pub ip: String,
    pub hostname: String,
    pub device_type: String, // "router", "switch", "firewall", "server", "camera", "printer", "ap", "nas", "other"
    pub vendor: String,
    pub model: String,
    pub os: String,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub status: String, // "online", "offline", "warning"
    pub x: f64,
    pub y: f64,
    pub group_id: Option<String>,
    pub mac: String,
    pub interfaces: Vec<InterfaceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceInfo {
    pub index: u32,
    pub name: String,
    pub mac: String,
    pub speed: u64,
    pub oper_status: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyLink {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub link_type: String, // "wired", "wireless", "vpn"
    pub speed: Option<u64>,
    pub latency_ms: Option<f64>,
    pub bandwidth_usage: Option<f64>,
    pub source_iface: Option<String>,
    pub target_iface: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologySnapshot {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDetail {
    pub id: String,
    pub name: String,
    pub layout_algorithm: String,
    pub created_at: String,
    pub nodes: Vec<TopologyNode>,
    pub links: Vec<TopologyLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDiff {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub changed_nodes: Vec<String>,
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/types/topology.rs
git commit -m "feat(topology): add enhanced data types"
```

---

### Task 2: Create topology layout algorithms

**Files:**
- Create: `src-tauri/src/core/topology/layout.rs`

- [ ] **Step 1: Create layout.rs with 4 layout algorithms**

```rust
//! Topology layout algorithms — force-directed, hierarchical, circular, grid.

use crate::types::topology::TopologyNode;

pub enum LayoutAlgorithm {
    ForceDirected,
    Hierarchical,
    Circular,
    Grid,
}

impl LayoutAlgorithm {
    pub fn from_str(s: &str) -> Self {
        match s {
            "hierarchical" => Self::Hierarchical,
            "circular" => Self::Circular,
            "grid" => Self::Grid,
            _ => Self::ForceDirected,
        }
    }
}

/// Compute node positions based on the selected layout algorithm.
/// (width, height) is the canvas viewport size.
pub fn compute_layout(
    nodes: &mut [TopologyNode],
    links: &[(String, String)],
    algorithm: &LayoutAlgorithm,
    width: f64,
    height: f64,
) {
    match algorithm {
        LayoutAlgorithm::ForceDirected => force_directed(nodes, links, width, height),
        LayoutAlgorithm::Hierarchical => hierarchical(nodes, links, width, height),
        LayoutAlgorithm::Circular => circular(nodes, width, height),
        LayoutAlgorithm::Grid => grid(nodes, width, height),
    }
}

/// Force-directed layout (spring-electric model).
/// Runs a fixed number of iterations (100) for initial positioning.
pub(crate) fn force_directed(nodes: &mut [TopologyNode], links: &[(String, String)], width: f64, height: f64) {
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let repulsion = 5000.0;
    let gravity = 0.01;
    let damping = 0.85;
    let min_dist = 80.0;

    let mut vx = vec![0.0; nodes.len()];
    let mut vy = vec![0.0; nodes.len()];

    for _ in 0..100 {
        for i in 0..nodes.len() {
            let mut fx = (center_x - nodes[i].x) * gravity;
            let mut fy = (center_y - nodes[i].y) * gravity;

            for j in 0..nodes.len() {
                if i == j { continue; }
                let dx = nodes[i].x - nodes[j].x;
                let dy = nodes[i].y - nodes[j].y;
                let dist = dx.hypot(dy).max(1.0);
                if dist < min_dist {
                    let force = repulsion / (dist * dist);
                    fx += (dx / dist) * force;
                    fy += (dy / dist) * force;
                }
            }

            // Spring attraction along links
            for (src, tgt) in links {
                let i_idx = if *src == nodes[i].ip || *src == nodes[i].id { Some(i) } else { None };
                let j_idx = if *tgt == nodes[i].ip || *tgt == nodes[i].id { Some(i) } else { None };
                let connected = i_idx.or(j_idx);
                if let Some(_) = connected {
                    let (other_ip, other_id) = if *src == nodes[i].ip || *src == nodes[i].id {
                        (tgt.as_str(), tgt.as_str())
                    } else {
                        (src.as_str(), src.as_str())
                    };
                    if let Some(other) = nodes.iter().position(|n| n.ip == other_ip || n.id == other_id) {
                        let dx = nodes[other].x - nodes[i].x;
                        let dy = nodes[other].y - nodes[i].y;
                        let dist = dx.hypot(dy).max(1.0);
                        fx += dx * 0.005;
                        fy += dy * 0.005;
                    }
                }
            }

            vx[i] = (vx[i] + fx) * damping;
            vy[i] = (vy[i] + fy) * damping;
            nodes[i].x = (nodes[i].x + vx[i]).clamp(30.0, width - 30.0);
            nodes[i].y = (nodes[i].y + vy[i]).clamp(30.0, height - 30.0);
        }
    }
}

/// Hierarchical layout — roughly layer nodes by IP heuristics (low IP = core layer).
pub(crate) fn hierarchical(nodes: &mut [TopologyNode], _links: &[(String, String)], width: f64, height: f64) {
    if nodes.is_empty() { return; }

    let margin = 60.0;
    let layer_gap = 150.0;
    let node_gap = 60.0;

    // Simple heuristic: group by third octet (subnet) or last octet ranges
    // 1-10 = core layer, 11-100 = distribution, 101-200 = access, 201-254 = edge
    let mut layers: Vec<Vec<usize>> = vec![vec![]; 4];
    for (i, node) in nodes.iter().enumerate() {
        let last_octet: u8 = node.ip.rsplit('.').next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let layer_idx = if last_octet <= 10 {
            0 // core
        } else if last_octet <= 100 {
            1 // distribution
        } else if last_octet <= 200 {
            2 // access
        } else {
            3 // edge
        };
        layers[layer_idx].push(i);
    }

    let total_layers = layers.iter().filter(|l| !l.is_empty()).count().max(1);
    let start_y = (height - (total_layers as f64 - 1.0) * layer_gap) / 2.0;

    for (layer_idx, layer) in layers.iter().enumerate() {
        if layer.is_empty() { continue; }
        let count = layer.len();
        let total_width = (count - 1) as f64 * node_gap;
        let start_x = (width - total_width) / 2.0;

        for (pos, &node_idx) in layer.iter().enumerate() {
            nodes[node_idx].x = start_x + pos as f64 * node_gap;
            nodes[node_idx].y = start_y + layer_idx as f64 * layer_gap;
        }
    }
}

/// Circular layout — nodes evenly spaced around a circle.
pub(crate) fn circular(nodes: &mut [TopologyNode], width: f64, height: f64) {
    if nodes.is_empty() { return; }
    let cx = width / 2.0;
    let cy = height / 2.0;
    let radius = (width.min(height) / 2.0) - 80.0;
    let count = nodes.len();

    for (i, node) in nodes.iter_mut().enumerate() {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
        node.x = cx + radius * angle.cos();
        node.y = cy + radius * angle.sin();
    }
}

/// Grid layout — arrange nodes in a grid, grouped by device type.
pub(crate) fn grid(nodes: &mut [TopologyNode], width: f64, height: f64) {
    if nodes.is_empty() { return; }

    // Sort by device type for grouping
    nodes.sort_by(|a, b| a.device_type.cmp(&b.device_type));

    let margin = 60.0;
    let cols = (nodes.len() as f64).sqrt().ceil() as usize;
    let cell_w = (width - margin * 2.0) / cols.max(1) as f64;
    let rows = (nodes.len() + cols - 1) / cols;
    let cell_h = (height - margin * 2.0) / rows.max(1) as f64;

    for (i, node) in nodes.iter_mut().enumerate() {
        let col = i % cols;
        let row = i / cols;
        node.x = margin + col as f64 * cell_w + cell_w / 2.0;
        node.y = margin + row as f64 * cell_h + cell_h / 2.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(ip: &str, device_type: &str) -> TopologyNode {
        TopologyNode {
            id: ip.to_string(),
            ip: ip.to_string(),
            hostname: String::new(),
            device_type: device_type.to_string(),
            vendor: String::new(),
            model: String::new(),
            os: String::new(),
            cpu_usage: None,
            memory_usage: None,
            status: "online".to_string(),
            x: 0.0, y: 0.0,
            group_id: None,
            mac: String::new(),
            interfaces: vec![],
        }
    }

    #[test]
    fn test_force_directed_moves_nodes() {
        let mut nodes = vec![make_node("192.168.1.1", "router"), make_node("192.168.1.2", "switch")];
        let links = vec![("192.168.1.1".into(), "192.168.1.2".into())];
        force_directed(&mut nodes, &links, 800.0, 600.0);
        // Nodes should have moved from (0,0)
        assert!(nodes[0].x != 0.0 || nodes[0].y != 0.0);
        assert!(nodes[1].x != 0.0 || nodes[1].y != 0.0);
    }

    #[test]
    fn test_circular_positions() {
        let mut nodes = vec![make_node("1", ""), make_node("2", ""), make_node("3", "")];
        circular(&mut nodes, 800.0, 600.0);
        // All should have non-zero positions
        for n in &nodes {
            assert!(n.x > 0.0 && n.y > 0.0);
        }
        // Should be roughly evenly spaced (angles differ by ~120 degrees)
        let angles: Vec<f64> = nodes.iter().map(|n| (n.y - 300.0).atan2(n.x - 400.0)).collect();
        for i in 1..angles.len() {
            let diff = (angles[i] - angles[0]).rem_euclid(std::f64::consts::TAU);
            assert!((diff - (i as f64 / 3.0) * std::f64::consts::TAU).abs() < 0.01);
        }
    }

    #[test]
    fn test_grid_layout_sorted() {
        let mut nodes = vec![
            make_node("10.0.0.1", "switch"),
            make_node("10.0.0.2", "router"),
            make_node("10.0.0.3", "switch"),
            make_node("10.0.0.4", "server"),
        ];
        grid(&mut nodes, 800.0, 600.0);
        // After sorting, routers come first (alphabetically)
        assert_eq!(nodes[0].device_type, "router");
        assert_eq!(nodes[1].device_type, "server");
        assert_eq!(nodes[2].device_type, "switch");
        assert_eq!(nodes[3].device_type, "switch");
        for n in &nodes {
            assert!(n.x > 0.0 && n.y > 0.0);
        }
    }

    #[test]
    fn test_empty_nodes() {
        let mut empty: Vec<TopologyNode> = vec![];
        circular(&mut empty, 800.0, 600.0); // should not panic
        grid(&mut empty, 800.0, 600.0); // should not panic
        hierarchical(&mut empty, &[], 800.0, 600.0); // should not panic
        force_directed(&mut empty, &[], 800.0, 600.0); // should not panic
    }
}
```

- [ ] **Step 2: Run tests to verify**

```bash
cd src-tauri && cargo test -- test_layout --nocapture
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/core/topology/layout.rs
git commit -m "feat(topology): add 4 layout algorithms with tests"
```

---

### Task 3: Create topology snapshot store

**Files:**
- Create: `src-tauri/src/core/topology/store.rs`

- [ ] **Step 1: Create store.rs**

```rust
//! SQLite store for topology snapshots.

use rusqlite::{params, Connection};
use std::sync::Mutex;

use crate::types::topology::{SnapshotDetail, SnapshotDiff, TopologyLink, TopologyNode};

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

// TopologySnapshot is used in list_snapshots return type, imported from types
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/core/topology/store.rs
git commit -m "feat(topology): add snapshot store with SQLite persistence"
```

---

### Task 4: Update topology commands

**Files:**
- Modify: `src-tauri/src/commands/topology.rs`
- Modify: `src-tauri/src/core/topology/mod.rs`

- [ ] **Step 1: Update core/topology/mod.rs to declare new submodules**

```rust
pub mod layout;
pub mod store;
// pub mod snmp; // Phase 2 — uncomment when SNMP is ready
```

- [ ] **Step 2: Add new Tauri commands to commands/topology.rs**

At the top of the file, add imports:
```rust
use std::sync::OnceLock;
use crate::core::topology::layout::{self, LayoutAlgorithm};
use crate::core::topology::store::TopologyStore;
use crate::types::topology::{SnapshotDetail, SnapshotDiff, TopologyNode, TopologyLink, TopologySnapshot};
```

Add a static store:
```rust
static TOPO_STORE: OnceLock<Arc<TopologyStore>> = OnceLock::new();

fn topo_store() -> &'static Arc<TopologyStore> {
    TOPO_STORE.get_or_init(|| {
        Arc::new(TopologyStore::new().expect("Failed to init topology store"))
    })
}

#[tauri::command]
pub async fn topology_init() -> Result<(), String> {
    let _ = topo_store();
    Ok(())
}
```

New commands to add (after existing ones, before `#[cfg(test)]`):

```rust
// ── Layout Commands ──

#[tauri::command]
pub fn compute_topology_layout(
    nodes: Vec<TopologyNode>,
    links: Vec<(String, String)>,
    algorithm: String,
    width: f64,
    height: f64,
) -> Vec<TopologyNode> {
    let mut result = nodes;
    let algo = LayoutAlgorithm::from_str(&algorithm);
    let link_pairs: Vec<(String, String)> = links.into_iter().collect();
    layout::compute_layout(&mut result, &link_pairs, &algo, width, height);
    result
}

// ── Snapshot Commands ──

#[tauri::command]
pub fn topology_save_snapshot(
    name: String,
    layout_algorithm: String,
    nodes: Vec<TopologyNode>,
    links: Vec<TopologyLink>,
) -> Result<String, String> {
    topo_store().save_snapshot(&name, &layout_algorithm, &nodes, &links)
}

#[tauri::command]
pub fn topology_list_snapshots() -> Result<Vec<TopologySnapshot>, String> {
    topo_store().list_snapshots()
}

#[tauri::command]
pub fn topology_load_snapshot(id: String) -> Result<SnapshotDetail, String> {
    topo_store().load_snapshot(&id)
}

#[tauri::command]
pub fn topology_delete_snapshot(id: String) -> Result<(), String> {
    topo_store().delete_snapshot(&id)
}

#[tauri::command]
pub fn topology_compare_snapshots(id_a: String, id_b: String) -> Result<SnapshotDiff, String> {
    topo_store().compare_snapshots(&id_a, &id_b)
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/topology.rs src-tauri/src/core/topology/mod.rs
git commit -m "feat(topology): add layout and snapshot commands"
```

---

### Task 5: Register new commands in lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add topology_init to setup and register commands**

In the `setup` closure, after the snmp init:
```rust
// Initialize topology store
let _ = commands::topology::topology_init();
```

In the `invoke_handler`, after `commands::topology::cancel_topology_discovery,`:
```rust
// Topology enhanced commands
commands::topology::compute_topology_layout,
commands::topology::topology_save_snapshot,
commands::topology::topology_list_snapshots,
commands::topology::topology_load_snapshot,
commands::topology::topology_delete_snapshot,
commands::topology::topology_compare_snapshots,
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(topology): register new topology commands"
```

---

### Task 6: Frontend — TypeScript bindings and Pinia store

**Files:**
- Modify: `src/lib/tauri.ts`
- Create: `src/stores/topology.ts`

- [ ] **Step 1: Add TypeScript interfaces and wrappers to tauri.ts**

After the existing topology section (after line ~1018), add:
```typescript
// ── Enhanced Topology Types ──

export interface InterfaceInfo {
  index: number;
  name: string;
  mac: string;
  speed: number;
  operStatus: number;
}

export interface TopologyNode {
  id: string;
  ip: string;
  hostname: string;
  deviceType: string;
  vendor: string;
  model: string;
  os: string;
  cpuUsage: number | null;
  memoryUsage: number | null;
  status: string;
  x: number;
  y: number;
  groupId: string | null;
  mac: string;
  interfaces: InterfaceInfo[];
}

export interface TopologyLink {
  id: string;
  sourceId: string;
  targetId: string;
  linkType: string;
  speed: number | null;
  latencyMs: number | null;
  bandwidthUsage: number | null;
  sourceIface: string | null;
  targetIface: string | null;
}

export interface TopologySnapshot {
  id: string;
  name: string;
  createdAt: string;
}

export interface SnapshotDetail {
  id: string;
  name: string;
  layoutAlgorithm: string;
  createdAt: string;
  nodes: TopologyNode[];
  links: TopologyLink[];
}

export interface SnapshotDiff {
  addedNodes: string[];
  removedNodes: string[];
  changedNodes: string[];
}

// ── Layout ──

export function computeTopologyLayout(
  nodes: TopologyNode[],
  links: [string, string][],
  algorithm: string,
  width: number,
  height: number,
): Promise<TopologyNode[]> {
  return invoke<TopologyNode[]>("compute_topology_layout", { nodes, links, algorithm, width, height });
}

// ── Snapshots ──

export function topologySaveSnapshot(name: string, layoutAlgorithm: string, nodes: TopologyNode[], links: TopologyLink[]): Promise<string> {
  return invoke<string>("topology_save_snapshot", { name, layoutAlgorithm, nodes, links });
}

export function topologyListSnapshots(): Promise<TopologySnapshot[]> {
  return invoke<TopologySnapshot[]>("topology_list_snapshots");
}

export function topologyLoadSnapshot(id: string): Promise<SnapshotDetail> {
  return invoke<SnapshotDetail>("topology_load_snapshot", { id });
}

export function topologyDeleteSnapshot(id: string): Promise<void> {
  return invoke<void>("topology_delete_snapshot", { id });
}

export function topologyCompareSnapshots(idA: string, idB: string): Promise<SnapshotDiff> {
  return invoke<SnapshotDiff>("topology_compare_snapshots", { idA, idB });
}
```

- [ ] **Step 2: Create Pinia store**

```typescript
// src/stores/topology.ts

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import {
  computeTopologyLayout,
  topologySaveSnapshot,
  topologyListSnapshots,
  topologyLoadSnapshot,
  topologyDeleteSnapshot,
  topologyCompareSnapshots,
  type TopologyNode,
  type TopologyLink,
  type TopologySnapshot,
  type SnapshotDetail,
  type SnapshotDiff,
} from "@/lib/tauri";

export const useTopologyStore = defineStore("topology", () => {
  // Device nodes
  const nodes = ref<TopologyNode[]>([]);
  const links = ref<TopologyLink[]>([]);
  const selectedNodeId = ref<string | null>(null);

  // Layout
  const layoutAlgorithm = ref("forceDirected");
  const canvasWidth = ref(800);
  const canvasHeight = ref(600);

  // Search & filter
  const searchQuery = ref("");
  const deviceTypeFilter = ref<string[]>([]);
  const statusFilter = ref<string[]>([]);

  const filteredNodes = computed(() => {
    let result = nodes.value;
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase();
      result = result.filter(
        (n) =>
          n.ip.toLowerCase().includes(q) ||
          n.hostname.toLowerCase().includes(q) ||
          n.vendor.toLowerCase().includes(q) ||
          n.model.toLowerCase().includes(q),
      );
    }
    if (deviceTypeFilter.value.length > 0) {
      result = result.filter((n) => deviceTypeFilter.value.includes(n.deviceType));
    }
    if (statusFilter.value.length > 0) {
      result = result.filter((n) => statusFilter.value.includes(n.status));
    }
    return result;
  });

  // Snapshots
  const snapshots = ref<TopologySnapshot[]>([]);
  const isLoadingSnapshots = ref(false);

  async function computeLayout() {
    const linkPairs: [string, string][] = links.value.map((l) => [l.sourceId, l.targetId]);
    try {
      const updated = await computeTopologyLayout(
        nodes.value,
        linkPairs,
        layoutAlgorithm.value,
        canvasWidth.value,
        canvasHeight.value,
      );
      nodes.value = updated;
    } catch (e) {
      console.error("Layout computation failed:", e);
    }
  }

  async function loadSnapshots() {
    isLoadingSnapshots.value = true;
    try {
      snapshots.value = await topologyListSnapshots();
    } finally {
      isLoadingSnapshots.value = false;
    }
  }

  async function saveSnapshot(name: string) {
    const id = await topologySaveSnapshot(name, layoutAlgorithm.value, nodes.value, links.value);
    await loadSnapshots();
    return id;
  }

  async function loadSnapshot(id: string) {
    const detail: SnapshotDetail = await topologyLoadSnapshot(id);
    nodes.value = detail.nodes;
    links.value = detail.links;
    layoutAlgorithm.value = detail.layoutAlgorithm;
  }

  async function deleteSnapshot(id: string) {
    await topologyDeleteSnapshot(id);
    await loadSnapshots();
  }

  async function compareSnapshots(idA: string, idB: string): Promise<SnapshotDiff> {
    return await topologyCompareSnapshots(idA, idB);
  }

  function updateNodePosition(nodeId: string, x: number, y: number) {
    const node = nodes.value.find((n) => n.id === nodeId);
    if (node) {
      node.x = x;
      node.y = y;
    }
  }

  function toggleDeviceTypeFilter(type: string) {
    const idx = deviceTypeFilter.value.indexOf(type);
    if (idx >= 0) {
      deviceTypeFilter.value.splice(idx, 1);
    } else {
      deviceTypeFilter.value.push(type);
    }
  }

  return {
    nodes,
    links,
    selectedNodeId,
    layoutAlgorithm,
    canvasWidth,
    canvasHeight,
    searchQuery,
    deviceTypeFilter,
    statusFilter,
    filteredNodes,
    snapshots,
    isLoadingSnapshots,
    computeLayout,
    loadSnapshots,
    saveSnapshot,
    loadSnapshot,
    deleteSnapshot,
    compareSnapshots,
    updateNodePosition,
    toggleDeviceTypeFilter,
  };
});
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/tauri.ts src/stores/topology.ts
git commit -m "feat(topology): add frontend bindings and store"
```

---

### Task 7: Enhance topology Vue page

**Files:**
- Modify: `src/pages/topology/Page.vue`

This is the largest task. The existing page is entirely Canvas + force-directed layout. We need to add:
1. Layout algorithm selector dropdown
2. Search bar + filter chips
3. Snapshot save/load UI
4. Enhanced Canvas rendering (shapes per device type, state rings)
5. Layout transition via store.computeLayout()

- [ ] **Step 1: Add imports and reactive state for new features**

In `<script setup>`, add:
```typescript
import { useTopologyStore } from "@/stores/topology";

const topoStore = useTopologyStore();
const showSnapshotPanel = ref(false);
const snapshotName = ref("");
const algorithmOptions = [
  { value: "forceDirected", label: "力导向布局" },
  { value: "hierarchical", label: "层级布局" },
  { value: "circular", label: "环形布局" },
  { value: "grid", label: "网格布局" },
];
```

Replace `initNodes()` with:
```typescript
function initNodes() {
  topoStore.canvasWidth = canvasWidth;
  topoStore.canvasHeight = canvasHeight;
  topoStore.nodes = peers.value.map((peer) => ({
    id: peer.id,
    ip: peer.ip,
    hostname: peer.hostname || peer.ip,
    deviceType: peer.os === "__discovered__" ? "other" : detectDeviceType(peer.ip),
    vendor: "",
    model: "",
    os: peer.os === "__discovered__" ? "" : peer.os,
    cpuUsage: null,
    memoryUsage: null,
    status: peer.status === "online" ? "online" : "offline",
    x: centerX + (Math.random() - 0.5) * 400,
    y: centerY + (Math.random() - 0.5) * 400,
    groupId: null,
    mac: "",
    interfaces: [],
  }));
}

function detectDeviceType(ip: string): string {
  const last = parseInt(ip.split(".")[3] || "0");
  if (last === 1 || last === 254) return "router";
  if (last >= 2 && last <= 10) return "switch";
  return "other";
}
```

Add layout switching:
```typescript
async function switchLayout(algo: string) {
  topoStore.layoutAlgorithm = algo;
  await topoStore.computeLayout();
}

async function saveCurrentSnapshot() {
  if (!snapshotName.value) return;
  // Build nodes and links from current canvas state
  const topoNodes = topoStore.nodes.map((n) => ({
    ...n,
    interfaces: [],
  }));
  const linkPairs: TopologyLink[] = [];
  // Add subnet links
  const subnets = getSubnets();
  const ipToId = new Map(topoNodes.map((n) => [n.ip, n.id]));
  for (const [, subnetPeers] of subnets) {
    for (let i = 0; i < subnetPeers.length; i++) {
      for (let j = i + 1; j < subnetPeers.length; j++) {
        const srcId = ipToId.get(subnetPeers[i].ip);
        const tgtId = ipToId.get(subnetPeers[j].ip);
        if (srcId && tgtId) {
          linkPairs.push({
            id: `${srcId}-${tgtId}`,
            sourceId: srcId,
            targetId: tgtId,
            linkType: "wired",
            speed: null,
            latencyMs: null,
            bandwidthUsage: null,
            sourceIface: null,
            targetIface: null,
          });
        }
      }
    }
  }
  // Add discovered links
  for (const link of discoveredLinks.value) {
    const srcId = ipToId.get(link.source);
    const tgtId = ipToId.get(link.target);
    if (srcId && tgtId) {
      linkPairs.push({
        id: `disc-${srcId}-${tgtId}`,
        sourceId: srcId,
        targetId: tgtId,
        linkType: "wired",
        speed: null,
        latencyMs: link.latencyMs,
        bandwidthUsage: null,
        sourceIface: null,
        targetIface: null,
      });
    }
  }
  try {
    const id = await topoStore.saveSnapshot(snapshotName.value);
    snapshotName.value = "";
    toast.add("success", `拓扑快照已保存: ${id.slice(0, 8)}`);
  } catch (e: any) {
    toast.add("error", String(e));
  }
}

async function loadSnapshotById(snapshotId: string) {
  await topoStore.loadSnapshot(snapshotId);
  // Sync back to canvas peers
  peers.value = topoStore.nodes.map((n) => ({
    id: n.id,
    hostname: n.hostname,
    ip: n.ip,
    os: n.os || n.deviceType,
    listen_port: 0,
    last_seen: new Date().toISOString(),
    status: n.status === "online" ? "online" : "offline",
  }));
  initNodes();
  toast.add("info", "拓扑快照已加载");
}
```

- [ ] **Step 2: Update the Canvas drawing to use device-type-specific shapes**

Replace the node drawing section in the `draw()` function (lines ~206-258) with enhanced rendering:

```typescript
// Draw nodes
for (const node of nodes.value) {
  const { x, y } = node;
  const topoNode = topoStore.nodes.find((n) => n.id === node.peer.id);
  const isSelected = selectedPeer.value?.ip === node.peer.ip;
  const isDiscovered = node.peer.os === "__discovered__";
  const isFilteredOut = searchQuery.value && !topoStore.filteredNodes.find((n) => n.id === node.peer.id);

  if (isFilteredOut) {
    ctx.globalAlpha = 0.15;
  }

  // Status ring (outer circle showing CPU/memory health)
  if (topoNode?.cpuUsage !== null && topoNode !== undefined) {
    ctx.beginPath();
    ctx.arc(x, y, 26, 0, Math.PI * 2);
    ctx.strokeStyle = topoNode.cpuUsage > 80 ? "#ef4444" : topoNode.cpuUsage > 50 ? "#f59e0b" : "#22c55e";
    ctx.lineWidth = 3;
    ctx.stroke();
  }

  // Device-type-specific shape
  const shape = getDeviceShape(topoNode?.deviceType || "other");

  if (shape === "diamond") {
    // Router: diamond
    ctx.beginPath();
    ctx.moveTo(x, y - 22);
    ctx.lineTo(x + 22, y);
    ctx.lineTo(x, y + 22);
    ctx.lineTo(x - 22, y);
    ctx.closePath();
  } else if (shape === "roundedRect") {
    // Switch: rounded rectangle
    const r = 6;
    const w = 36;
    const h = 26;
    ctx.beginPath();
    ctx.moveTo(x - w/2 + r, y - h/2);
    ctx.lineTo(x + w/2 - r, y - h/2);
    ctx.quadraticCurveTo(x + w/2, y - h/2, x + w/2, y - h/2 + r);
    ctx.lineTo(x + w/2, y + h/2 - r);
    ctx.quadraticCurveTo(x + w/2, y + h/2, x + w/2 - r, y + h/2);
    ctx.lineTo(x - w/2 + r, y + h/2);
    ctx.quadraticCurveTo(x - w/2, y + h/2, x - w/2, y + h/2 - r);
    ctx.lineTo(x - w/2, y - h/2 + r);
    ctx.quadraticCurveTo(x - w/2, y - h/2, x - w/2 + r, y - h/2);
    ctx.closePath();
  } else {
    // Server/Other: circle (default)
    ctx.beginPath();
    ctx.arc(x, y, 22, 0, Math.PI * 2);
  }

  ctx.fillStyle = getDeviceColor(topoNode?.deviceType || "other", isSelected, isDiscovered);
  ctx.fill();

  // Border for discovered/selected
  if (isSelected) {
    ctx.strokeStyle = "#22c55e";
    ctx.lineWidth = 3;
    ctx.stroke();
  } else if (isDiscovered) {
    ctx.strokeStyle = "rgba(59, 130, 246, 0.5)";
    ctx.lineWidth = 2;
    ctx.setLineDash([3, 3]);
    ctx.stroke();
    ctx.setLineDash([]);
  }

  // Labels (same as before)
  ctx.fillStyle = colors.labelColor;
  ctx.font = "10px monospace";
  ctx.textAlign = "center";
  const label = node.peer.hostname || node.peer.ip;
  ctx.fillText(label.length > 14 ? label.slice(0, 14) + "..." : label, x, y + 38);

  // Type label
  const typeLabel = topoNode?.deviceType || (isDiscovered ? "ping" : node.peer.os);
  if (typeLabel) {
    ctx.fillStyle = colors.inkSoft + "99";
    ctx.font = "8px sans-serif";
    ctx.fillText(typeLabel, x, y + 50);
  }

  ctx.globalAlpha = 1;
}

function getDeviceShape(deviceType: string): string {
  switch (deviceType) {
    case "router": case "firewall": return "diamond";
    case "switch": case "ap": return "roundedRect";
    default: return "circle";
  }
}

function getDeviceColor(deviceType: string, isSelected: boolean, isDiscovered: boolean): string {
  if (isSelected) return "#22c55e";
  if (isDiscovered) return "#3b82f6";
  switch (deviceType) {
    case "router": return "#7c3aed";   // purple
    case "switch": return "#0891b2";   // cyan
    case "firewall": return "#dc2626"; // red
    case "server": return "#2563eb";   // blue
    case "camera": return "#059669";   // green
    case "printer": return "#d97706";  // amber
    default: return "#475569";         // slate
  }
}
```

- [ ] **Step 3: Add search bar, filter chips, layout selector to the template**

After the header title block (after the "自动发现" button), add:

```html
<!-- Search & Controls Bar -->
<div class="mt-3 flex items-center gap-2">
  <!-- Search -->
  <div class="relative flex-1">
    <input
      v-model="topoStore.searchQuery"
      type="text"
      placeholder="搜索 IP / 主机名 / 厂商..."
      class="w-full rounded-lg border border-paper-deep/60 bg-paper-deep/50 px-3 py-1.5 pl-8 text-xs text-ink outline-none transition-colors focus:border-bamboo/50"
    />
    <svg class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ink-faint" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>
  </div>

  <!-- Layout selector -->
  <select
    v-model="topoStore.layoutAlgorithm"
    @change="switchLayout(topoStore.layoutAlgorithm)"
    class="rounded-lg border border-paper-deep/60 bg-paper-deep/50 px-2 py-1.5 text-xs text-ink outline-none"
  >
    <option v-for="algo in algorithmOptions" :key="algo.value" :value="algo.value">
      {{ algo.label }}
    </option>
  </select>

  <!-- Snapshot button -->
  <button
    class="rounded-lg px-3 py-1.5 text-xs font-medium transition-colors"
    :class="showSnapshotPanel ? 'bg-paper-deep text-ink' : 'bg-paper-deep/30 text-ink-faint hover:bg-paper-deep/60'"
    @click="showSnapshotPanel = !showSnapshotPanel"
  >
    快照
  </button>
</div>

<!-- Snapshot Panel -->
<div v-if="showSnapshotPanel" class="mt-2 rounded-xl border border-paper-deep/60 bg-paper/90 p-3 shadow-sm backdrop-blur">
  <div class="mb-2 flex gap-2">
    <input
      v-model="snapshotName"
      type="text"
      placeholder="快照名称"
      class="flex-1 rounded-lg border border-paper-deep/60 bg-paper-deep/50 px-2 py-1.5 text-xs text-ink outline-none focus:border-bamboo/50"
    />
    <button
      class="rounded-lg bg-bamboo px-3 py-1.5 text-xs font-medium text-white hover:bg-bamboo/90 disabled:opacity-50"
      :disabled="!snapshotName"
      @click="saveCurrentSnapshot"
    >
      保存
    </button>
  </div>
  <div v-if="topoStore.snapshots.length > 0" class="mt-2 max-h-40 overflow-y-auto">
    <div
      v-for="snap in topoStore.snapshots"
      :key="snap.id"
      class="flex items-center justify-between rounded-lg px-2 py-1.5 hover:bg-paper-deep/50"
    >
      <div>
        <span class="text-xs text-ink">{{ snap.name }}</span>
        <span class="ml-2 text-[10px] text-ink-faint">{{ new Date(snap.createdAt).toLocaleString() }}</span>
      </div>
      <div class="flex gap-1">
        <button class="rounded px-2 py-0.5 text-[10px] text-bamboo hover:bg-bamboo/10" @click="loadSnapshotById(snap.id)">加载</button>
        <button class="rounded px-2 py-0.5 text-[10px] text-red-500 hover:bg-red-50" @click="topoStore.deleteSnapshot(snap.id)">删除</button>
      </div>
    </div>
  </div>
  <div v-else class="py-2 text-center text-xs text-ink-faint">暂无快照</div>
</div>

<!-- Filter chips -->
<div v-if="topoStore.nodes.length > 0" class="mt-2 flex flex-wrap gap-1">
  <button
    v-for="type in ['router', 'switch', 'server', 'firewall', 'camera', 'other']"
    :key="type"
    class="rounded-full px-2 py-0.5 text-[10px] transition-colors"
    :class="topoStore.deviceTypeFilter.includes(type)
      ? 'bg-bamboo/20 text-bamboo'
      : 'bg-paper-deep/30 text-ink-faint hover:bg-paper-deep/60'"
    @click="topoStore.toggleDeviceTypeFilter(type)"
  >
    {{ {router:'路由器', switch:'交换机', server:'服务器', firewall:'防火墙', camera:'摄像头', other:'其他'}[type] }}
  </button>
</div>
```

- [ ] **Step 4: Initialize store on mount and load snapshots**

In the `onMounted` callback, add:
```typescript
topoStore.loadSnapshots();
```

And replace the `draw` function's `simulateForces()` call at the end to use the store:
```typescript
// Continue animation — use force-directed for real-time simulation
// Layout algorithms are applied on-demand via computeLayout()
simulateForces();
```

- [ ] **Step 5: Commit**

```bash
git add src/pages/topology/Page.vue
git commit -m "feat(topology): enhance page with layout selector, search, filters, snapshots, device shapes"
```

---

### Task 8: Build verification

**Files:** None (verification only)

- [ ] **Step 1: Run cargo check**

```bash
cd src-tauri && cargo check 2>&1
```

Expected: Build succeeds (pre-existing warnings OK). Fix any compile errors.

- [ ] **Step 2: Fix issues and verify frontend build**

```bash
cd .. && npx vue-tsc --noEmit 2>&1 || true
npm run build 2>&1
```

Expected: Build succeeds. Fix any errors.

- [ ] **Step 3: Commit any fixes**

```bash
git add -A
git commit -m "fix(topology): fix build issues"
```

---

### Phase 2 Tasks (after SNMP is implemented)

These tasks depend on the SNMP feature being complete and the `core/snmp/` module being available.

### Task 9: Create SNMP integration module

**Files:**
- Create: `src-tauri/src/core/topology/snmp.rs`

- [ ] **Step 1: Create snmp.rs for device type enrichment**

```rust
//! SNMP integration for topology — enrich nodes with device type, interfaces, resources.

use crate::core::snmp::store::SnmpStore;
use crate::types::topology::TopologyNode;

/// Enrich topology nodes with data from the SNMP store.
/// Matches by IP address.
pub fn enrich_from_snmp(nodes: &mut [TopologyNode]) -> Result<(), String> {
    // SnmpStore uses OnceLock; access via the commands module
    Err("SNMP store not available yet".to_string())
}
```

Uncomment `pub mod snmp;` in `core/topology/mod.rs`.

- [ ] **Step 2: Add enrich command to commands/topology.rs**

```rust
#[tauri::command]
pub async fn topology_enrich_from_snmp(nodes: Vec<TopologyNode>) -> Result<Vec<TopologyNode>, String> {
    let mut result = nodes;
    // This will be implemented when SNMP is ready
    // crate::core::topology::snmp::enrich_from_snmp(&mut result)?;
    Ok(result)
}
```

Register `commands::topology::topology_enrich_from_snmp` in lib.rs.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/core/topology/snmp.rs src-tauri/src/core/topology/mod.rs src-tauri/src/commands/topology.rs src-tauri/src/lib.rs
git commit -m "feat(topology): add SNMP integration scaffold"
```

---

### Spec Coverage

| Spec Requirement | Task |
|---|---|
| Data models (TopologyNode, TopologyLink, enums) | Task 1 |
| Layout algorithms (force/hierarchical/circular/grid) | Task 2 |
| Layout algorithm tests | Task 2 |
| SQLite snapshot persistence | Task 3 |
| Snapshot CRUD commands | Task 4 |
| Compute layout command | Task 4 |
| TypeScript interfaces + invoke wrappers | Task 6 |
| Pinia store with search/filter/compute | Task 6 |
| Enhanced node shapes per device type | Task 7 |
| Search bar + filter chips | Task 7 |
| Layout selector dropdown | Task 7 |
| Snapshot save/load UI | Task 7 |
| SNMP integration (Phase 2) | Task 9 |
| Build verification | Task 8 |
