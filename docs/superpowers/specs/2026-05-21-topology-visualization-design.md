# 网络拓扑可视化增强设计

## 概述

在 AzurePath 现有的网络拓扑页面基础上，从设备分类、布局算法、交互增强、拓扑持久化四个维度进行全面增强，打造内网拓扑可视化核心功能。

## 现有基础

- Canvas 力导向图绘制
- Ping 扫描自动发现设备（`commands/topology.rs`）
- 同子网聚簇线 + 发现连接线
- 节点详情弹窗、缩放、拖拽

## 增强架构

### 数据模型 (`types/topology.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyNode {
    pub id: String,
    pub ip: String,
    pub hostname: String,
    pub device_type: DeviceType,       // Router, Switch, Firewall, Server, Camera, Printer, Other
    pub vendor: String,
    pub model: String,
    pub os: String,
    pub cpu_usage: Option<f32>,        // 来自 SNMP collector
    pub memory_usage: Option<f32>,
    pub status: DeviceStatus,          // Online, Offline, Warning
    pub interfaces: Vec<InterfaceInfo>,// 来自 SNMP
    pub x: f64,                        // 已保存的坐标
    pub y: f64,
    pub group_id: Option<String>,      // 分组 ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyLink {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub link_type: LinkType,           // Wired, Wireless, VPN
    pub speed: Option<u64>,            // bps
    pub latency_ms: Option<f64>,
    pub bandwidth_usage: Option<f64>,  // 当前带宽利用率 %
    pub source_iface: Option<String>,  // 源端接口名（SNMP）
    pub target_iface: Option<String>,  // 目标端接口名
}

pub enum DeviceType { Router, Switch, Firewall, Server, Camera, Printer, Ap, Nas, Other }
pub enum DeviceStatus { Online, Offline, Warning }
pub enum LinkType { Wired, Wireless, Vpn }
pub enum LayoutAlgorithm { ForceDirected, Hierarchical, Circular, Grid }
```

### 核心模块 (`core/topology/`)

| 文件 | 职责 |
|------|------|
| `mod.rs` | 节点/连接管理、SNMP 数据合并 |
| `layout.rs` | 四种布局算法实现 |
| `store.rs` | SQLite 持久化（拓扑快照、节点布局） |
| `snmp.rs` | SNMP 数据集成（设备类型识别、接口信息、资源监控） |

### 布局算法 (`core/topology/layout.rs`)

1. **ForceDirected** — 现有的力导向布局（保留为默认）
2. **Hierarchical** — 按设备角色分层（核心 .1/.2 → 汇聚 → 接入 → 终端），基于 BFS 层级分配 + 层内力导向
3. **Circular** — 节点沿圆环均匀分布，可拖动调整顺序
4. **Grid** — 网格排列，按设备类型分区域排列

### 设备类型识别 (`core/topology/snmp.rs`)

对接 SNMP 模块（`core/snmp/`）：
- 通过设备 IP 查询 SNMP 数据库中的 `sysObjectId` 和 `sysDescr`
- 映射 OID 前缀到设备类型：Cisco/Huawei/H3C = Switch/Router, Hikvision = Camera, HP = Printer 等
- 获取接口列表（`ifTable`）关联到节点，用于链路识别
- 周期同步 CPU/内存数据到节点状态

### 持久化 (`core/topology/store.rs`)

```sql
CREATE TABLE topology_snapshots (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    layout TEXT NOT NULL,         -- JSON: 节点坐标、缩放、布局算法
    created_at TEXT NOT NULL
);

CREATE TABLE topology_nodes_saved (
    snapshot_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    group_id TEXT,
    PRIMARY KEY (snapshot_id, node_id)
);
```

### 命令层 (`commands/topology.rs`)

新增命令（保留现有 `discover_topology` / `cancel_topology_discovery`）：

```rust
// 布局控制
set_topology_layout(algorithm: String) -> ()
set_node_position(node_id: String, x: f64, y: f64) -> ()

// 快照管理
save_topology_snapshot(name: String) -> String    // 返回 id
load_topology_snapshot(id: String) -> SnapshotDetail
list_topology_snapshots() -> Vec<TopologySummary>
delete_topology_snapshot(id: String) -> ()
compare_topology_snapshots(id_a: String, id_b: String) -> SnapshotDiff

// SNMP 集成
enrich_topology_from_snmp() -> ()                  // 从 SNMP 数据库丰富节点信息
```

### 前端增强 (`src/pages/topology/Page.vue`)

**控制栏（已有面板基础上增强）：**
- 布局选择器：下拉菜单切换四种布局
- 搜索输入框：实时高亮匹配节点
- 过滤按钮组：按设备类型/状态过滤
- 快照操作：保存/加载/对比

**Canvas 渲染增强：**
- 节点形状：根据 `deviceType` 绘制不同形状（圆形、菱形、圆角方形、三角形）
- 节点状态环：根据 CPU/内存/在线状态显示不同颜色环
- 链路样式：根据类型和带宽利用率显示不同颜色和粗细
- Tooltip：鼠标悬停显示节点/链路概要信息
- 动画过渡：切换布局时节点位置插值动画

**交互增强：**
- 右键上下文菜单：跳转 Ping / 远程桌面 / 设备详情
- 多选 + 批量操作
- 拖拽分组：将节点拖入分组区域

## 数据流

```
SNMP 扫描/手动发现
       │
       ▼
  topology:discover  ──→  SNMP 数据库
       │                       │
       ▼                       ▼
  拓扑图节点/链路 ←── enrich_from_snmp()
       │
       ▼
  用户调整布局/保存快照
       │
       ▼
  SQLite (topology_snapshots)
```

## 依赖关系

- 依赖 SNMP 模块：设备类型识别、接口数据、资源监控（可选增强，非必须）
- 独立功能：布局算法、持久化、搜索过滤可以独立于 SNMP 先实现
