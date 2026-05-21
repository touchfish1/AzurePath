//! Topology layout algorithms -- force-directed, hierarchical, circular, grid.

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
                        let _dist = dx.hypot(dy).max(1.0);
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

/// Hierarchical layout -- roughly layer nodes by IP heuristics (low IP = core layer).
pub(crate) fn hierarchical(nodes: &mut [TopologyNode], _links: &[(String, String)], width: f64, height: f64) {
    if nodes.is_empty() { return; }

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

/// Circular layout -- nodes evenly spaced around a circle.
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

/// Grid layout -- arrange nodes in a grid, grouped by device type.
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
