# SNMP 网络管理功能设计

## 概述

在 AzurePath 中添加 SNMP v2c 支持，实现网络设备发现、接口监控、性能数据采集等功能，使运维人员能够通过 SNMP 协议管理和监控内网设备。

## 技术栈

- `snmp2` — 纯 Rust SNMP v2c 实现，无外部依赖
- `rusqlite` — 存储采集的历史数据（项目中已使用）
- Tauri Event — 实时推送采集数据到前端

## 三层架构

### 数据模型 (`types/snmp.rs`)

```rust
/// SNMP 连接配置
pub struct SnmpSessionConfig {
    pub host: String,
    pub port: u16,        // 默认 161
    pub community: String,
    pub timeout_ms: u64,  // 默认 3000
}

/// SNMP 发现设备
pub struct SnmpDevice {
    pub id: String,
    pub ip: String,
    pub hostname: String,
    pub sys_descr: String,       // 系统描述
    pub sys_object_id: String,    // 设备类型 OID
    pub vendor: String,           // 厂商
    pub model: String,            // 型号
    pub uptime: u64,              // 运行时间
    pub last_seen: String,
}

/// 网络接口
pub struct SnmpInterface {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub mac: String,
    pub ip: String,
    pub speed: u64,           // bps
    pub admin_status: u8,     // 1=up, 2=down
    pub oper_status: u8,
    pub in_octets: u64,
    pub out_octets: u64,
}

/// 采集样本
pub struct SnmpSample {
    pub device_id: String,
    pub timestamp: String,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub interfaces: Vec<InterfaceSample>,
}

pub struct InterfaceSample {
    pub index: u32,
    pub in_bps: f64,
    pub out_bps: f64,
    /// 对比前一次采样的差值
}
```

### 核心模块 (`core/snmp/`)

| 文件 | 职责 |
|------|------|
| `mod.rs` | `SnmpSession` — 封装 snmp2 的 get/walk/getbulk 操作 |
| `oids.rs` | 常用 OID 常量（sysName, sysDescr, ifTable, ..1.1.2 等） |
| `scanner.rs` | 子网扫描，并发探测存活 SNMP 设备 |
| `collector.rs` | 定时采集器，轮询接口流量 + CPU/内存 |
| `store.rs` | SQLite 持久化（设备信息、历史采样数据） |

### 命令层 (`commands/snmp.rs`)

```rust
// 设备发现
discover_snmp_devices(cidr: String, community: String) -> Vec<SnmpDevice>

// 设备详情
get_snmp_device(device_id: String) -> SnmpDevice
delete_snmp_device(device_id: String) -> ()

// 接口查询
get_snmp_interfaces(device_id: String) -> Vec<SnmpInterface>

// ARP/路由表
get_snmp_arp_table(device_id: String) -> Vec<ArpEntry>
get_snmp_route_table(device_id: String) -> Vec<RouteEntry>

// 采集控制
start_snmp_collector(device_id: String, interval_secs: u64) -> ()
stop_snmp_collector(device_id: String) -> ()

// 历史数据
get_snmp_history(device_id: String, range: String) -> Vec<SnmpSample>
```

## 数据流

1. 用户输入 CIDR + community，触发 `discover_snmp_devices`
2. `Scanner` 并发地向每个 IP 发送 `sysDescr` 请求，识别 SNMP 设备
3. 发现结果存入 SQLite `snmp_devices` 表
4. `Collector` 按间隔轮询已保存设备的接口计数器（ifHCInOctets/ifHCOutOctets）
5. 计算差值得到 bps，写入 `snmp_samples` 表
6. 实时数据通过 `snmp:sample` 事件推送到前端

## 前端页面

### 发现页 (`src/pages/snmp/Page.vue`)
- 输入框：目标 CIDR、Community（默认 public）
- "扫描"按钮 + 进度条
- 设备列表（IP、主机名、型号、厂商）
- 点击设备进入详情页

### 设备详情页（同一路由，参数切换）
- 基本信息卡片（系统描述、运行时间、型号）
- 标签页：接口 / CPU内存 / 流量趋势 / ARP表 / 路由表
- 流量趋势使用 Canvas/SVG 折线图

## 持久化

```sql
CREATE TABLE snmp_devices (
    id TEXT PRIMARY KEY,
    ip TEXT NOT NULL,
    hostname TEXT,
    sys_descr TEXT,
    sys_object_id TEXT,
    vendor TEXT,
    model TEXT,
    uptime INTEGER,
    last_seen TEXT NOT NULL,
    community TEXT NOT NULL
);

CREATE TABLE snmp_interfaces (
    device_id TEXT NOT NULL,
    if_index INTEGER NOT NULL,
    name TEXT,
    description TEXT,
    mac TEXT,
    ip TEXT,
    speed INTEGER,
    admin_status INTEGER,
    oper_status INTEGER,
    PRIMARY KEY (device_id, if_index)
);

CREATE TABLE snmp_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    if_index INTEGER NOT NULL,
    in_bps REAL,
    out_bps REAL,
    cpu_usage REAL,
    memory_usage REAL
);
```

## OID 参考

| OID | 名称 | 用途 |
|-----|------|------|
| `1.3.6.1.2.1.1.1.0` | sysDescr | 系统描述 |
| `1.3.6.1.2.1.1.5.0` | sysName | 主机名 |
| `1.3.6.1.2.1.1.3.0` | sysUpTime | 运行时间 |
| `1.3.6.1.2.1.2.2.1.2` | ifDescr | 接口描述 |
| `1.3.6.1.2.1.2.2.1.6` | ifPhysAddress | 接口 MAC |
| `1.3.6.1.2.1.2.2.1.8` | ifOperStatus | 接口状态 |
| `1.3.6.1.2.1.2.2.1.10` | ifInOctets | 入流量 |
| `1.3.6.1.2.1.2.2.1.16` | ifOutOctets | 出流量 |
| `1.3.6.1.2.1.4.22.1.2` | ipNetToMediaPhysAddress | ARP MAC |
| `1.3.6.1.2.1.4.22.1.3` | ipNetToMediaNetAddress | ARP IP |
| `1.3.6.1.2.1.25.3.3.1.2` | hrProcessorLoad | CPU 使用率 |
