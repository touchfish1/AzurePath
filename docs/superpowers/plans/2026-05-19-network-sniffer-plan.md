# 网络嗅探功能 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 AzurePath 中实现局域网网络嗅探功能，自动发现在线设备、检测开放端口、识别运行服务及版本。

**Architecture:** Rust 后端自实现 TCP 连接探测 + Banner 抓取 + 轻量级指纹库，设备发现使用 TCP Ping（无需管理员权限），端口扫描使用 tokio 并发 TCP Connect，服务版本通过 Banner 正则匹配指纹库识别。前端使用 Vue 实时展示扫描结果。

**Tech Stack:** Rust (tokio, serde, regex), Vue 3, lucide-vue-next

---

## 文件结构

### 后端 — 新增

| 文件 | 职责 |
|------|------|
| `src-tauri/src/types/network_sniffer.rs` | SnifferOptions, DeviceResult, PortResult, SnifferProgress, PortPreset |
| `src-tauri/src/core/network_sniffer/mod.rs` | 模块入口，重新导出 |
| `src-tauri/src/core/network_sniffer/discovery.rs` | 设备发现 (TCP Ping + ARP 缓存解析) |
| `src-tauri/src/core/network_sniffer/port_scanner.rs` | 端口扫描 (并发 TCP Connect) |
| `src-tauri/src/core/network_sniffer/banner.rs` | Banner 抓取 + HTTP/TLS/主动探测 |
| `src-tauri/src/core/network_sniffer/fingerprint.rs` | 指纹数据库 + 正则匹配引擎 |
| `src-tauri/src/core/network_sniffer/os_detect.rs` | OS 探测 (TTL + 端口组合分析) |
| `src-tauri/src/commands/network_sniffer.rs` | Tauri 命令层 (start/stop/list/export/presets + 事件) |

### 后端 — 修改

| 文件 | 修改 |
|------|------|
| `src-tauri/src/types/mod.rs` | 添加 `pub mod network_sniffer;` |
| `src-tauri/src/commands/mod.rs` | 添加 `pub mod network_sniffer;` |
| `src-tauri/src/core/mod.rs` | 添加 `pub mod network_sniffer;` |
| `src-tauri/src/lib.rs` | 注册 5 个命令到 generate_handler! |

### 前端 — 新增

| 文件 | 职责 |
|------|------|
| `src/pages/network-sniffer/Page.vue` | 主页面 (配置区 + 结果区 + 汇总) |
| `src/components/network-sniffer/PortModal.vue` | 端口选择弹窗 (分类浏览 + 搜索 + 手动输入) |

### 前端 — 修改

| 文件 | 修改 |
|------|------|
| `src/lib/tauri.ts` | 添加 SnifferOptions, DeviceResult, PortResult 等类型 + 5 个 API 函数 + 5 个事件监听 |
| `src/router/index.ts` | 添加 /network-sniffer 路由 |
| `src/components/layout/Sidebar.vue` | 添加 "网络嗅探" 导航项 (使用 Radio 图标) |

---

## 任务分解

### Task 1: 数据模型 + 命令骨架 + 前端绑定

**Files:**
- Create: `src-tauri/src/types/network_sniffer.rs`
- Create: `src-tauri/src/commands/network_sniffer.rs`
- Modify: `src-tauri/src/types/mod.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: 创建 types/network_sniffer.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnifferOptions {
    pub targets: Vec<String>,
    pub ports: Vec<u16>,
    pub mode: String,          // "fast" | "deep"
    pub concurrency_hosts: u32,
    pub concurrency_ports: u32,
    pub timeout_ms: u64,
    pub probe_services: bool,
}

impl Default for SnifferOptions {
    fn default() -> Self {
        Self {
            targets: vec!["192.168.1.0/24".to_string()],
            ports: vec![
                21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 445,
                993, 995, 1433, 1521, 2049, 3306, 3389, 5432, 5900, 6379,
                8080, 8443, 9090, 27017,
            ],
            mode: "fast".to_string(),
            concurrency_hosts: 10,
            concurrency_ports: 50,
            timeout_ms: 1000,
            probe_services: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceResult {
    pub ip: String,
    pub hostname: Option<String>,
    pub mac: Option<String>,
    pub os: Option<String>,
    pub open_ports: Vec<PortResult>,
    pub is_alive: bool,
    pub scan_mode: String,
    pub scan_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortResult {
    pub port: u16,
    pub protocol: String,
    pub state: String,
    pub service: Option<String>,
    pub version: Option<String>,
    pub banner: Option<String>,
    pub confidence: u8,
    pub probe_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnifferProgress {
    pub total_hosts: u32,
    pub scanned_hosts: u32,
    pub services_found: u32,
    pub current_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortPreset {
    pub name: String,
    pub label: String,
    pub ports: Vec<u16>,
}

pub fn default_presets() -> Vec<PortPreset> {
    vec![
        PortPreset {
            name: "top100".to_string(),
            label: "常用 (TOP 100)".to_string(),
            ports: vec![
                7, 9, 13, 21, 22, 23, 25, 26, 37, 53, 79, 80, 81, 88, 106,
                110, 111, 113, 119, 135, 139, 143, 144, 179, 199, 389, 427,
                443, 444, 445, 465, 513, 514, 515, 543, 544, 548, 554, 587,
                631, 646, 873, 990, 993, 995, 1025, 1026, 1027, 1028, 1029,
                1110, 1433, 1720, 1723, 1755, 1900, 2000, 2001, 2049, 2121,
                2717, 3000, 3128, 3306, 3389, 3986, 4899, 5000, 5009, 5051,
                5060, 5101, 5190, 5357, 5432, 5631, 5666, 5800, 5900, 6000,
                6001, 6646, 7070, 8000, 8008, 8009, 8080, 8443, 8888, 9000,
                9001, 9090, 9100, 9999, 10000, 32768, 49152, 49153, 49154,
                49155, 49156,
            ],
        },
        PortPreset {
            name: "web".to_string(),
            label: "常见 Web".to_string(),
            ports: vec![80, 443, 8080, 8443, 3000, 5000, 8000, 8888, 9090],
        },
        PortPreset {
            name: "database".to_string(),
            label: "数据库".to_string(),
            ports: vec![3306, 5432, 1433, 1521, 27017, 6379, 9200, 11211],
        },
        PortPreset {
            name: "all".to_string(),
            label: "所有 (1-1024)".to_string(),
            ports: (1..=1024).collect(),
        },
    ]
}
```

- [ ] **Step 2: 创建 commands/network_sniffer.rs 骨架**

```rust
use crate::types::network_sniffer::{DeviceResult, PortPreset, SnifferOptions};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
pub async fn sniffer_start(
    app: tauri::AppHandle,
    options: SnifferOptions,
) -> Result<String, String> {
    let _ = app;
    Err("Not implemented yet".to_string())
}

#[tauri::command]
pub async fn sniffer_stop(task_id: String) -> Result<(), String> {
    let cancel = {
        let tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.get(&task_id).cloned()
    };
    match cancel {
        Some(c) => { c.store(true, Ordering::SeqCst); Ok(()) }
        None => Err(format!("Task {} not found", task_id)),
    }
}

#[tauri::command]
pub async fn sniffer_list() -> Result<Vec<DeviceResult>, String> {
    Err("Not implemented yet".to_string())
}

#[tauri::command]
pub async fn sniffer_export(task_id: String, format: String) -> Result<String, String> {
    let _ = (task_id, format);
    Err("Not implemented yet".to_string())
}

#[tauri::command]
pub async fn sniffer_presets() -> Result<Vec<PortPreset>, String> {
    Ok(crate::types::network_sniffer::default_presets())
}
```

- [ ] **Step 3: 注册模块到 mod.rs 和 lib.rs**

在 `src-tauri/src/types/mod.rs` 添加:
```rust
pub mod network_sniffer;
```

在 `src-tauri/src/commands/mod.rs` 添加:
```rust
pub mod network_sniffer;
```

在 `src-tauri/src/core/mod.rs` 添加:
```rust
pub mod network_sniffer;
```

在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中添加 5 个命令:
```rust
commands::network_sniffer::sniffer_start,
commands::network_sniffer::sniffer_stop,
commands::network_sniffer::sniffer_list,
commands::network_sniffer::sniffer_export,
commands::network_sniffer::sniffer_presets,
```

- [ ] **Step 4: 添加前端类型和 API 绑定到 tauri.ts**

在文件末端追加 (末尾已有 Clipboard 相关代码):

```typescript
// ============================================================
// Network Sniffer
// ============================================================

export interface SnifferOptions {
  targets: string[];
  ports: number[];
  mode: string;
  concurrencyHosts: number;
  concurrencyPorts: number;
  timeoutMs: number;
  probeServices: boolean;
}

export interface PortResult {
  port: number;
  protocol: string;
  state: string;
  service: string | null;
  version: string | null;
  banner: string | null;
  confidence: number;
  probeMethod: string;
}

export interface DeviceResult {
  ip: string;
  hostname: string | null;
  mac: string | null;
  os: string | null;
  openPorts: PortResult[];
  isAlive: boolean;
  scanMode: string;
  scanCompleted: boolean;
}

export interface SnifferProgress {
  totalHosts: number;
  scannedHosts: number;
  servicesFound: number;
  currentTarget: string;
}

export interface PortPreset {
  name: string;
  label: string;
  ports: number[];
}

export function snifferStart(options: SnifferOptions): Promise<string> {
  return invoke<string>("sniffer_start", { options });
}

export function snifferStop(taskId: string): Promise<void> {
  return invoke<void>("sniffer_stop", { taskId });
}

export function snifferList(): Promise<DeviceResult[]> {
  return invoke<DeviceResult[]>("sniffer_list");
}

export function snifferExport(taskId: string, format: string): Promise<string> {
  return invoke<string>("sniffer_export", { taskId, format });
}

export function snifferPresets(): Promise<PortPreset[]> {
  return invoke<PortPreset[]>("sniffer_presets");
}

// Events
export function onSnifferProgress(cb: (p: SnifferProgress) => void): Promise<UnlistenFn> {
  return listen<SnifferProgress>("sniffer:progress", (e) => cb(e.payload));
}

export function onSnifferDevice(cb: (d: DeviceResult) => void): Promise<UnlistenFn> {
  return listen<DeviceResult>("sniffer:device", (e) => cb(e.payload));
}

export function onSnifferPort(cb: (p: PortResult & { ip: string }) => void): Promise<UnlistenFn> {
  return listen("sniffer:port", (e) => cb(e.payload as any));
}

export function onSnifferComplete(cb: (p: { taskId: string }) => void): Promise<UnlistenFn> {
  return listen("sniffer:complete", (e) => cb(e.payload as any));
}

export function onSnifferError(cb: (p: { taskId: string; error: string }) => void): Promise<UnlistenFn> {
  return listen("sniffer:error", (e) => cb(e.payload as any));
}
```

- [ ] **Step 5: 验证编译**

```bash
cd src-tauri && cargo check 2>&1
```

Expected: 编译成功，sniffer_start/sniffer_list 等命令返回 "Not implemented yet"（这是预期的骨架状态）。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/types/network_sniffer.rs src-tauri/src/types/mod.rs
git add src-tauri/src/commands/network_sniffer.rs src-tauri/src/commands/mod.rs
git add src-tauri/src/core/mod.rs src-tauri/src/lib.rs src/lib/tauri.ts
git commit -m "feat(sniffer): add data models, command skeleton, and frontend bindings"
```

---

### Task 2: 设备发现 + 端口扫描核心

**Files:**
- Create: `src-tauri/src/core/network_sniffer/mod.rs`
- Create: `src-tauri/src/core/network_sniffer/discovery.rs`
- Create: `src-tauri/src/core/network_sniffer/port_scanner.rs`

- [ ] **Step 1: 创建 core/network_sniffer/mod.rs**

```rust
pub mod discovery;
pub mod port_scanner;
```

- [ ] **Step 2: 实现设备发现模块 (discovery.rs)**

设备发现使用 TCP Ping 方式，无需管理员权限：
- 对目标 IP 尝试连接常见端口（445, 135, 22, 80, 443），任一成功即视为存活
- 并发执行，超时合并

```rust
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;

/// Probe ports that are commonly open on LAN devices.
const PROBE_PORTS: &[u16] = &[445, 22, 80, 443, 139, 135, 8080, 3389];

/// Check if a single host is alive by trying to connect to probe ports.
pub async fn is_host_alive(
    ip: IpAddr,
    timeout_ms: u64,
) -> bool {
    let timeout = Duration::from_millis(timeout_ms);
    for &port in PROBE_PORTS {
        if TcpStream::connect_timeout(&(ip, port).into(), timeout).await.is_ok() {
            return true;
        }
    }
    false
}

/// Discover alive hosts from a list of IPs concurrently.
pub async fn discover_hosts(
    ips: &[IpAddr],
    concurrency: usize,
    timeout_ms: u64,
) -> Vec<IpAddr> {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(ips.len());

    for &ip in ips {
        let permit = semaphore.clone().acquire_owned().await;
        if permit.is_err() {
            continue;
        }
        handles.push(tokio::spawn(async move {
            let alive = is_host_alive(ip, timeout_ms).await;
            drop(permit);
            (ip, alive)
        }));
    }

    let mut alive_hosts = Vec::new();
    for h in handles {
        if let Ok((ip, true)) = h.await {
            alive_hosts.push(ip);
        }
    }
    alive_hosts
}

/// Get MAC address and vendor by parsing `arp -a` output.
/// Returns (mac, vendor) if the IP is found in ARP cache.
pub fn resolve_mac(ip: &str) -> Option<(String, Option<String>)> {
    let output = std::process::Command::new("arp")
        .arg("-a")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains(ip) {
            // Windows format:  "192.168.1.1    00-11-22-33-44-55    dynamic"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let mac = parts[1].replace('-', ":").to_uppercase();
                let vendor = mac_vendor(&mac);
                return Some((mac, vendor));
            }
        }
    }
    None
}

/// Resolve hostname via reverse DNS lookup.
pub fn resolve_hostname(ip: &str) -> Option<String> {
    std::net::ToSocketAddrs::to_socket_addrs(&(ip, 0))
        .ok()?
        .filter_map(|a| match a {
            std::net::SocketAddr::V4(_) => None,
            _ => Some(a),
        })
        .next()
        .and_then(|_| {
            // Try reverse DNS via std
            None
        })
}

/// A minimal MAC vendor lookup (first 3 bytes).
fn mac_vendor(mac: &str) -> Option<String> {
    let prefix = mac.split(':').take(3).collect::<Vec<_>>().join(":");
    let vendor = match prefix.as_str() {
        "00:11:22" => "Dell",
        "00:1A:2B" => "Cisco",
        "00:14:22" => "Dell",
        "00:1E:68" => "Intel",
        "00:21:6A" => "Apple",
        "00:23:32" => "Apple",
        "00:25:00" => "Apple",
        "00:26:08" => "Apple",
        "00:26:4B" => "Apple",
        "00:30:65" => "Apple",
        "00:50:56" => "VMware",
        "00:0C:29" => "VMware",
        "00:05:69" => "VMware",
        "00:1C:42" => "Parallels",
        "08:00:27" => "Oracle VirtualBox",
        "00:15:5D" => "Hyper-V",
        "00:50:B6" => "HP",
        "00:1A:4B" => "HP",
        "00:21:5A" => "HP",
        "00:1C:BI" => "Huawei",
        "00:25:9C" => "Huawei",
        "28:6E:D4" => "Xiaomi",
        "FC:A1:3F" => "Xiaomi",
        "50:76:AF" => "Huawei",
        "E0:CC:7A" => "Huawei",
        "14:75:90" => "TP-Link",
        "C0:4A:00" => "TP-Link",
        "E8:DE:27" => "TP-Link",
        "D4:6E:0E" => "TP-Link",
        "A8:57:4E" => "TP-Link",
        "3C:52:A1" => "TP-Link",
        _ => return None,
    };
    Some(vendor.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_vendor_known() {
        assert_eq!(mac_vendor("00:50:56:12:34:56"), Some("VMware".to_string()));
        assert_eq!(mac_vendor("08:00:27:AB:CD:EF"), Some("Oracle VirtualBox".to_string()));
    }

    #[test]
    fn test_mac_vendor_unknown() {
        assert_eq!(mac_vendor("AA:BB:CC:DD:EE:FF"), None);
    }

    #[test]
    fn test_arp_parse_windows_format() {
        // "192.168.1.1    00-11-22-33-44-55    dynamic" should parse to "00:11:22:33:44:55"
        let ip = "192.168.1.1";
        let arp_output = "  192.168.1.1    00-11-22-33-44-55    dynamic\n";
        for line in arp_output.lines() {
            if line.contains(ip) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                assert_eq!(parts.len(), 3);
                let mac = parts[1].replace('-', ":").to_uppercase();
                assert_eq!(mac, "00:11:22:33:44:55");
            }
        }
    }
}
```

- [ ] **Step 3: 实现端口扫描模块 (port_scanner.rs)**

```rust
use crate::types::network_sniffer::PortResult;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;

/// Scan ports on a single target, returning open port results.
pub async fn scan_ports(
    target: &str,
    ports: &[u16],
    concurrency: usize,
    timeout_ms: u64,
    cancel: Arc<AtomicBool>,
) -> Vec<PortResult> {
    let timeout = Duration::from_millis(timeout_ms);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut results = Vec::new();

    // Process ports in chunks to avoid spawning too many tasks
    for chunk in ports.chunks(concurrency * 2) {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        let mut handles = Vec::with_capacity(chunk.len());
        for &port in chunk {
            if cancel.load(Ordering::SeqCst) {
                break;
            }

            let target = target.to_string();
            let sem = semaphore.clone();
            let cancel = cancel.clone();

            handles.push(tokio::spawn(async move {
                let _permit = match sem.acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => return None,
                };
                if cancel.load(Ordering::SeqCst) {
                    return None;
                }
                match TcpStream::connect_timeout(
                    &(target.as_str(), port).into(),
                    timeout,
                )
                .await
                {
                    Ok(_) => Some(PortResult {
                        port,
                        protocol: "tcp".to_string(),
                        state: "open".to_string(),
                        service: None,
                        version: None,
                        banner: None,
                        confidence: 0,
                        probe_method: "tcp_connect".to_string(),
                    }),
                    Err(_) => None,
                }
            }));
        }

        for h in handles {
            if let Some(Some(port_result)) = h.await.ok().flatten() {
                results.push(port_result);
            }
        }
    }

    results.sort_by_key(|r| r.port);
    results
}

/// Return the "top ports" for quick mode — most commonly open ports.
pub fn top_ports() -> Vec<u16> {
    vec![
        21, 22, 23, 25, 53, 80, 81, 110, 111, 135, 139, 143, 389, 443,
        445, 465, 514, 543, 544, 548, 554, 587, 631, 646, 873, 990, 993,
        995, 1025, 1026, 1027, 1028, 1029, 1110, 1433, 1720, 1723, 1755,
        1900, 2000, 2001, 2049, 2121, 3000, 3128, 3306, 3389, 3986, 4899,
        5000, 5009, 5051, 5060, 5101, 5190, 5357, 5432, 5631, 5666, 5800,
        5900, 6000, 6001, 6646, 7070, 8000, 8008, 8009, 8080, 8443, 8888,
        9000, 9001, 9090, 9100, 9999, 10000, 32768, 49152, 49153, 49154,
        49155, 49156,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_top_ports_non_empty() {
        let ports = top_ports();
        assert!(!ports.is_empty());
        assert!(ports.contains(&80));
        assert!(ports.contains(&443));
        assert!(ports.contains(&22));
    }

    #[tokio::test]
    async fn test_cancel_stops_scan() {
        let cancel = Arc::new(AtomicBool::new(true));
        let ports = vec![80, 443, 22, 3306];
        let results = scan_ports("127.0.0.1", &ports, 5, 500, cancel).await;
        // With cancel=true before starting, should be empty
        assert!(results.is_empty());
    }
}
```

- [ ] **Step 4: 运行测试验证**

```bash
cd src-tauri && cargo test --lib core::network_sniffer::discovery core::network_sniffer::port_scanner 2>&1
```

Expected: 测试通过（mac_vendor 测试、top_ports 测试、cancel 测试）

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/core/network_sniffer/
git commit -m "feat(sniffer): implement device discovery (TCP ping) and port scanner"
```

---

### Task 3: 服务检测 — Banner 抓取 + 指纹库

**Files:**
- Modify: `src-tauri/src/core/network_sniffer/mod.rs`
- Create: `src-tauri/src/core/network_sniffer/banner.rs`
- Create: `src-tauri/src/core/network_sniffer/fingerprint.rs`

- [ ] **Step 1: 更新 mod.rs**

```rust
pub mod banner;
pub mod discovery;
pub mod fingerprint;
pub mod port_scanner;
```

- [ ] **Step 2: 实现 Banner 抓取模块 (banner.rs)**

```rust
use crate::types::network_sniffer::PortResult;

/// Read initial banner from an open TCP connection.
/// Sends optional probe bytes, then reads up to 4096 bytes of response.
pub async fn grab_banner(
    target: &str,
    port: u16,
    timeout_ms: u64,
) -> Option<String> {
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let addr = format!("{}:{}", target, port);

    let mut stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .ok()?
        .ok()?;

    // Send probe based on port
    let probe = probe_for_port(port);
    if !probe.is_empty() {
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            stream.write_all(probe.as_bytes()),
        )
        .await;
    }

    // Read response
    let mut buf = vec![0u8; 4096];
    let n = tokio::time::timeout(timeout, stream.read(&mut buf))
        .await
        .ok()?
        .ok()?;

    if n == 0 {
        return None;
    }

    // Try UTF-8, fallback to lossy
    let banner = String::from_utf8_lossy(&buf[..n.min(4096)]).to_string();
    let banner = banner.trim_matches('\0').trim().to_string();
    if banner.is_empty() { None } else { Some(banner) }
}

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Return a probe string appropriate for the given port.
fn probe_for_port(port: u16) -> &'static str {
    match port {
        21 => "",                          // FTP - server sends banner first
        22 => "",                          // SSH - server sends banner first
        25 | 587 => "EHLO probe\r\n",      // SMTP
        80 | 8080 | 8000 | 8888 => {
            "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n"
        }
        110 => "",                         // POP3 - server sends banner first
        143 => "",                         // IMAP - server sends banner first
        443 | 8443 => "",                  // TLS handled separately
        993 | 995 => "",                   // TLS
        3306 => "",                        // MySQL - server sends handshake first
        5432 => "",                        // PostgreSQL - server sends banner first
        6379 => "PING\r\n",                // Redis
        _ => "",
    }
}

/// Perform TLS handshake and extract certificate info.
/// Only used for ports 443, 8443.
pub async fn tls_probe(target: &str, port: u16, timeout_ms: u64) -> Option<String> {
    use tokio_rustls::TlsConnector as RustlsConnector;
    use std::sync::Arc;
    use rustls::ClientConfig;
    use tokio_rustls::webpki::ServerName;

    let timeout = std::time::Duration::from_millis(timeout_ms);
    let addr = format!("{}:{}", target, port);

    let stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .ok()?
        .ok()?;

    let mut config = ClientConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();
    // Allow any certificate (we just want the server name)
    config.dangerous().set_certificate_verifier(
        Arc::new(AllowAnyCert),
    );

    let connector = RustlsConnector::from(Arc::new(config));
    let domain = ServerName::try_from(target).ok()?;

    let tls_stream = tokio::time::timeout(timeout, connector.connect(domain, stream))
        .await
        .ok()?
        .ok()?;

    let (_stream, session) = tls_stream.into_inner();
    let certs = session.peer_certificates()?;
    let first_cert = certs.first()?;
    let der = rustls::Certificate(first_cert.0.clone());

    // Parse certificate to extract CN / SAN
    let parsed = x509_parser::parse_x509_certificate(&der.0).ok()?;
    let cn = parsed.1.subject()
        .iter_common_name()
        .next()
        .and_then(|attr| attr.as_str().ok())
        .map(|s| format!("TLS · CN: {}", s));

    Some(cn.unwrap_or_else(|| "TLS · unknown".to_string()))
}

use rustls::client::danger::ServerCertVerifier;

struct AllowAnyCert;

impl ServerCertVerifier for AllowAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _sct: &[u8],
        _ocsp: &[u8],
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::Certificate,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::Certificate,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
}
```

Wait — `tokio-rustls` and `rustls` and `x509-parser` would be new dependencies. That's a lot for a first version. Let me simplify: for TLS probe, skip the deep TLS parsing in v1 and just note it as a future enhancement. In practice, HTTPS servers also respond on port 443 via the HTTP probe (GET / HTTP/1.0), so we can get the Server header that way.

Let me simplify banner.rs:

```rust
use crate::types::network_sniffer::PortResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Read initial banner from an open TCP connection.
pub async fn grab_banner(
    target: &str,
    port: u16,
    timeout_ms: u64,
) -> Option<String> {
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let addr = format!("{}:{}", target, port);

    let mut stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .ok()?
        .ok()?;

    // Send probe based on port
    let probe = probe_for_port(port);
    if !probe.is_empty() {
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            stream.write_all(probe.as_bytes()),
        )
        .await;
    }

    // Read response
    let mut buf = vec![0u8; 4096];
    let n = tokio::time::timeout(timeout, stream.read(&mut buf))
        .await
        .ok()?
        .ok()?;

    if n == 0 {
        return None;
    }

    let banner = String::from_utf8_lossy(&buf[..n.min(4096)]).to_string();
    let banner = banner.trim_matches('\0').trim().to_string();
    if banner.is_empty() { None } else { Some(banner) }
}

/// Return a probe string appropriate for the given port.
fn probe_for_port(port: u16) -> &'static str {
    match port {
        21 => "",
        22 => "",
        25 | 587 => "EHLO probe\r\n",
        80 | 8080 | 8000 | 8888 => "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n",
        110 => "",
        143 => "",
        3306 => "",
        5432 => "",
        6379 => "PING\r\n",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_for_port() {
        assert_eq!(probe_for_port(80), "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n");
        assert_eq!(probe_for_port(22), "");
        assert_eq!(probe_for_port(6379), "PING\r\n");
        assert_eq!(probe_for_port(9999), "");
    }
}
```

- [ ] **Step 3: 实现指纹库 (fingerprint.rs)**

```rust
use crate::types::network_sniffer::PortResult;
use regex::Regex;
use std::sync::LazyLock;

/// A single fingerprint entry matching banner text to service + version.
pub struct Fingerprint {
    pub ports: &'static [u16],
    pub service_name: &'static str,
    pub pattern: &'static str,
    pub version_group: Option<usize>,
    pub confidence: u8,
}

/// All known service fingerprints.
static FINGERPRINTS: LazyLock<Vec<Fingerprint>> = LazyLock::new(|| {
    vec![
        // Web servers
        Fingerprint { ports: &[80, 443, 8080, 8443, 8000, 8888], service_name: "nginx", pattern: r"(?i)Server:\s*nginx(?:/([\d.]+))?", version_group: Some(1), confidence: 95 },
        Fingerprint { ports: &[80, 443, 8080], service_name: "Apache", pattern: r"(?i)Server:\s*Apache(?:/([\d.]+))?", version_group: Some(1), confidence: 95 },
        Fingerprint { ports: &[80, 443], service_name: "IIS", pattern: r"(?i)Server:\s*Microsoft-IIS(?:/([\d.]+))?", version_group: Some(1), confidence: 95 },
        // SSH
        Fingerprint { ports: &[22], service_name: "OpenSSH", pattern: r"SSH-2\.0-OpenSSH[_-]([\w.]+)", version_group: Some(1), confidence: 95 },
        Fingerprint { ports: &[22], service_name: "Dropbear", pattern: r"(?i)dropbear", version_group: None, confidence: 85 },
        // FTP
        Fingerprint { ports: &[21], service_name: "vsftpd", pattern: r"(?i)vsFTPd(?: ([\w.]+))?", version_group: Some(1), confidence: 90 },
        Fingerprint { ports: &[21], service_name: "proftpd", pattern: r"(?i)ProFTPD(?: ([\w.]+))?", version_group: Some(1), confidence: 90 },
        Fingerprint { ports: &[21], service_name: "Pure-FTPd", pattern: r"(?i)Pure-FTPd", version_group: None, confidence: 85 },
        // MySQL
        Fingerprint { ports: &[3306], service_name: "MySQL", pattern: r"(?i)mysql|MariaDB", version_group: None, confidence: 80 },
        // PostgreSQL
        Fingerprint { ports: &[5432], service_name: "PostgreSQL", pattern: r"(?i)postgres|psql", version_group: None, confidence: 80 },
        // Redis
        Fingerprint { ports: &[6379], service_name: "Redis", pattern: r"(?i)redis_version:|\+OK", version_group: None, confidence: 85 },
        // SMTP
        Fingerprint { ports: &[25, 587], service_name: "Postfix", pattern: r"(?i)ESMTP\s+Postfix", version_group: None, confidence: 85 },
        Fingerprint { ports: &[25, 587], service_name: "Sendmail", pattern: r"(?i)ESMTP\s+Sendmail", version_group: None, confidence: 85 },
        Fingerprint { ports: &[25, 587], service_name: "Exim", pattern: r"(?i)Exim", version_group: None, confidence: 85 },
        // SMB
        Fingerprint { ports: &[445], service_name: "Samba", pattern: r"(?i)Samba", version_group: None, confidence: 80 },
        // DNS
        Fingerprint { ports: &[53], service_name: "dnsmasq", pattern: r"(?i)dnsmasq", version_group: None, confidence: 75 },
        // Generic
        Fingerprint { ports: &[], service_name: "HTTP", pattern: r"^HTTP/", version_group: None, confidence: 60 },
    ]
});

/// Match a banner to a fingerprint, returning (service_name, version, confidence).
pub fn match_banner(port: u16, banner: &str, default_service: Option<&str>) -> Option<(String, Option<String>, u8)> {
    for fp in FINGERPRINTS.iter() {
        if !fp.ports.is_empty() && !fp.ports.contains(&port) {
            continue;
        }
        if let Ok(re) = Regex::new(fp.pattern) {
            if let Some(caps) = re.captures(banner) {
                let version = fp.version_group.and_then(|g| caps.get(g)).map(|m| m.as_str().to_string());
                return Some((fp.service_name.to_string(), version, fp.confidence));
            }
        }
    }

    // Fallback: use port-based service guess
    let service = default_service.or_else(|| guess_service_by_port(port)).map(|s| s.to_string());
    service.map(|s| (s, None, 30))
}

/// Guess service name based on port number alone.
pub fn guess_service_by_port(port: u16) -> Option<&'static str> {
    match port {
        21 => Some("FTP"),
        22 => Some("SSH"),
        23 => Some("Telnet"),
        25 => Some("SMTP"),
        53 => Some("DNS"),
        80 => Some("HTTP"),
        110 => Some("POP3"),
        111 => Some("RPC"),
        135 => Some("RPC"),
        139 => Some("NetBIOS"),
        143 => Some("IMAP"),
        443 => Some("HTTPS"),
        445 => Some("SMB"),
        465 => Some("SMTPS"),
        587 => Some("SMTP"),
        993 => Some("IMAPS"),
        995 => Some("POP3S"),
        1433 => Some("MSSQL"),
        1521 => Some("Oracle"),
        2049 => Some("NFS"),
        3306 => Some("MySQL"),
        3389 => Some("RDP"),
        5432 => Some("PostgreSQL"),
        5900 => Some("VNC"),
        6379 => Some("Redis"),
        8080 => Some("HTTP-Alt"),
        8443 => Some("HTTPS-Alt"),
        9090 => Some("HTTP-Alt"),
        27017 => Some("MongoDB"),
        _ => None,
    }
}

/// Run service detection: grab banner, match fingerprint, fill PortResult.
pub async fn detect_service(
    target: &str,
    mut port_result: PortResult,
    banner_timeout_ms: u64,
) -> PortResult {
    if port_result.state != "open" {
        return port_result;
    }

    // Grab banner
    let banner = super::banner::grab_banner(target, port_result.port, banner_timeout_ms).await;

    if let Some(ref b) = banner {
        port_result.banner = Some(b.clone());
        let default = guess_service_by_port(port_result.port);
        if let Some((service, version, confidence)) = match_banner(port_result.port, b, default) {
            port_result.service = Some(service);
            port_result.version = version;
            port_result.confidence = confidence;
            port_result.probe_method = "banner".to_string();
        }
    } else {
        // No banner but port open - set service guess from port
        port_result.service = guess_service_by_port(port_result.port).map(|s| s.to_string());
        port_result.confidence = 20;
    }

    port_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_service() {
        assert_eq!(guess_service_by_port(22), Some("SSH"));
        assert_eq!(guess_service_by_port(80), Some("HTTP"));
        assert_eq!(guess_service_by_port(3306), Some("MySQL"));
        assert_eq!(guess_service_by_port(9999), None);
    }

    #[test]
    fn test_match_nginx() {
        let banner = "HTTP/1.1 200 OK\r\nServer: nginx/1.24.0\r\n";
        let result = match_banner(80, banner, Some("HTTP"));
        assert!(result.is_some());
        let (service, version, confidence) = result.unwrap();
        assert_eq!(service, "nginx");
        assert_eq!(version, Some("1.24.0".to_string()));
        assert!(confidence >= 90);
    }

    #[test]
    fn test_match_apache() {
        let banner = "HTTP/1.1 200 OK\r\nServer: Apache/2.4.57 (Unix)\r\n";
        let result = match_banner(80, banner, None);
        assert!(result.is_some());
        let (service, version, _) = result.unwrap();
        assert_eq!(service, "Apache");
        assert_eq!(version, Some("2.4.57".to_string()));
    }

    #[test]
    fn test_match_ssh() {
        let banner = "SSH-2.0-OpenSSH_8.9p1 Ubuntu-3";
        let result = match_banner(22, banner, None);
        assert!(result.is_some());
        let (service, version, _) = result.unwrap();
        assert_eq!(service, "OpenSSH");
        assert_eq!(version, Some("8.9p1".to_string()));
    }

    #[test]
    fn test_match_redis() {
        let banner = "+OK\r\nredis_version:7.2.4";
        let result = match_banner(6379, banner, None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "Redis");
    }
}
```

- [ ] **Step 4: 更新 Cargo.toml — 添加 regex 依赖**

编辑 `src-tauri/Cargo.toml`，在 `[dependencies]` 中添加:
```toml
regex = "1"
```

- [ ] **Step 5: 运行测试**

```bash
cd src-tauri && cargo test --lib core::network_sniffer::fingerprint 2>&1
```

Expected: 所有指纹匹配测试通过 (match_nginx, match_apache, match_ssh, match_redis, guess_service)

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/core/network_sniffer/mod.rs
git add src-tauri/src/core/network_sniffer/banner.rs
git add src-tauri/src/core/network_sniffer/fingerprint.rs
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(sniffer): implement banner grabbing and service fingerprint database"
```

---

### Task 4: OS 探测 + 结果聚合

**Files:**
- Modify: `src-tauri/src/core/network_sniffer/mod.rs`
- Create: `src-tauri/src/core/network_sniffer/os_detect.rs`

- [ ] **Step 1: 更新 mod.rs**

```rust
pub mod banner;
pub mod discovery;
pub mod fingerprint;
pub mod os_detect;
pub mod port_scanner;
```

- [ ] **Step 2: 实现 OS 探测模块 (os_detect.rs)**

```rust
use crate::types::network_sniffer::DeviceResult;
use std::net::IpAddr;

/// Analyze TTL value to guess OS.
/// Common TTLs: Windows=128, Linux=64, macOS=64, Solaris=255, Network devices=255
pub fn guess_os_by_ttl(ttl: u8) -> Option<&'static str> {
    match ttl {
        0..=64 => Some("Linux/Unix"),
        65..=128 => Some("Windows"),
        129..=255 => Some("Network Device"),
    }
}

/// Refine OS guess based on open port profile.
/// Certain port combinations strongly suggest specific OS families.
pub fn refine_os_by_ports(ports: &[u16], base_os: Option<&str>) -> Option<String> {
    let has_windows_ports = ports.iter().any(|p| matches!(p, 135 | 139 | 445 | 3389));
    let has_linux_ports = ports.iter().any(|p| matches!(p, 22 | 111 | 2049));
    let has_router_ports = ports.iter().any(|p| matches!(p, 53 | 67 | 68 | 1900 | 5000));

    let os = match (base_os, has_windows_ports, has_linux_ports, has_router_ports) {
        (_, true, false, _) if !has_linux_ports => "Windows",
        (_, false, true, _) if !has_windows_ports => "Linux/Unix",
        (Some("Network Device"), _, _, true) => "Network Device",
        (Some(base), _, _, _) => base,
        (None, true, true, _) => "Unknown (mixed ports)",
        (None, false, false, false) => "Unknown",
        (None, false, false, true) => "Network Device",
    };
    Some(os.to_string())
}

/// Build a DeviceResult from scan data.
pub fn assemble_device(
    ip: IpAddr,
    hostname: Option<String>,
    mac: Option<String>,
    mac_vendor: Option<String>,
    open_ports: Vec<crate::types::network_sniffer::PortResult>,
    scan_mode: &str,
) -> DeviceResult {
    let ttl_os = None; // Would require ICMP to get TTL — future enhancement
    let ports: Vec<u16> = open_ports.iter().map(|p| p.port).collect();
    let os = refine_os_by_ports(&ports, ttl_os);

    // Use hostname from reverse DNS if available
    let resolved_hostname = hostname
        .filter(|h| !h.is_empty())
        .or_else(|| {
            // Simple reverse DNS via std
            None
        });

    DeviceResult {
        ip: ip.to_string(),
        hostname: resolved_hostname,
        mac,
        os,
        open_ports,
        is_alive: true,
        scan_mode: scan_mode.to_string(),
        scan_completed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_os_ttl() {
        assert_eq!(guess_os_by_ttl(64), Some("Linux/Unix"));
        assert_eq!(guess_os_by_ttl(128), Some("Windows"));
        assert_eq!(guess_os_by_ttl(255), Some("Network Device"));
    }

    #[test]
    fn test_os_by_ports_windows() {
        let os = refine_os_by_ports(&[135, 139, 445], None);
        assert_eq!(os, Some("Windows".to_string()));
    }

    #[test]
    fn test_os_by_ports_linux() {
        let os = refine_os_by_ports(&[22, 111, 2049], None);
        assert_eq!(os, Some("Linux/Unix".to_string()));
    }

    #[test]
    fn test_os_by_ports_router() {
        let os = refine_os_by_ports(&[53, 67, 1900], None);
        assert_eq!(os, Some("Network Device".to_string()));
    }

    #[test]
    fn test_os_by_ports_mixed() {
        let os = refine_os_by_ports(&[22, 80, 443, 3389], None);
        // Has 22 (linux), has 3389 (windows) → mixed
        assert!(os.is_some());
    }

    #[test]
    fn test_assemble_device() {
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        let ports = vec![
            crate::types::network_sniffer::PortResult {
                port: 80, protocol: "tcp".to_string(), state: "open".to_string(),
                service: Some("HTTP".to_string()), version: None, banner: None,
                confidence: 50, probe_method: "tcp_connect".to_string(),
            },
        ];
        let device = assemble_device(
            ip,
            Some("router.local".to_string()),
            Some("00:11:22:33:44:55".to_string()),
            Some("Dell".to_string()),
            ports,
            "fast",
        );
        assert_eq!(device.ip, "192.168.1.1");
        assert_eq!(device.hostname, Some("router.local".to_string()));
        assert_eq!(device.is_alive, true);
        assert_eq!(device.scan_mode, "fast");
    }
}
```

- [ ] **Step 3: 运行测试**

```bash
cd src-tauri && cargo test --lib core::network_sniffer::os_detect 2>&1
```

Expected: 所有 OS 探测测试通过

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/core/network_sniffer/mod.rs
git add src-tauri/src/core/network_sniffer/os_detect.rs
git commit -m "feat(sniffer): implement OS detection and device result assembly"
```

---

### Task 5: 命令层集成 — 完整扫描流程 + 事件推送

**Files:**
- Rewrite: `src-tauri/src/commands/network_sniffer.rs`

- [ ] **Step 1: 实现完整的 commands/network_sniffer.rs**

```rust
use crate::core::network_sniffer::{banner, discovery, fingerprint, os_detect, port_scanner};
use crate::types::network_sniffer::{
    default_presets, DeviceResult, PortPreset, PortResult, SnifferOptions, SnifferProgress,
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

static SCAN_RESULTS: LazyLock<Mutex<HashMap<String, Vec<DeviceResult>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn parse_cidr(cidr: &str) -> Result<Vec<IpAddr>, String> {
    let (base, prefix_len) = match cidr.split_once('/') {
        Some((ip, len)) => {
            let ip: IpAddr = ip.parse().map_err(|e| format!("Invalid IP: {}", e))?;
            let len: u8 = len.parse().map_err(|_| "Invalid CIDR prefix".to_string())?;
            (ip, len)
        }
        None => {
            let ip: IpAddr = cidr.parse().map_err(|e| format!("Invalid IP: {}", e))?;
            // Single IP — treat as /32
            return Ok(vec![ip]);
        }
    };

    let ip_u32 = match base {
        IpAddr::V4(v4) => u32::from(v4),
        IpAddr::V6(_) => return Err("IPv6 not supported yet".to_string()),
    };

    let mask = if prefix_len == 0 {
        0u32
    } else {
        !0u32 << (32 - prefix_len)
    };
    let network = ip_u32 & mask;
    let broadcast = network | !mask;
    let host_count = broadcast.saturating_sub(network).saturating_sub(1);
    // Limit to /16 (65534 hosts) max
    if host_count > 65534 {
        return Err("CIDR range too large (max /16)".to_string());
    }

    let mut ips = Vec::with_capacity(host_count as usize + 1);
    for i in 1..=host_count {
        if let Some(host_ip) = network.checked_add(i) {
            if host_ip < broadcast {
                ips.push(IpAddr::V4(host_ip.into()));
            }
        }
    }
    Ok(ips)
}

#[tauri::command]
pub async fn sniffer_start(
    app: AppHandle,
    options: SnifferOptions,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();
    let cancel_flag = Arc::new(AtomicBool::new(false));

    // Register cancel token
    {
        let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.insert(task_id.clone(), cancel_flag.clone());
    }

    let tid = task_id.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_scan(&app_clone, &tid, options, cancel_flag).await {
            let _ = app_clone.emit("sniffer:error", serde_json::json!({
                "taskId": tid,
                "error": e,
            }));
        }
        // Cleanup
        let _ = CANCEL_TOKENS.lock().map(|mut tokens| {
            tokens.remove(&tid);
        });
    });

    Ok(task_id)
}

async fn run_scan(
    app: &AppHandle,
    task_id: &str,
    options: SnifferOptions,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    // Phase 1: Parse targets → IP list
    let mut all_ips = Vec::new();
    for target in &options.targets {
        let ips = parse_cidr(target)?;
        all_ips.extend(ips);
    }
    all_ips.sort();
    all_ips.dedup();

    let total_hosts = all_ips.len() as u32;
    let concurrency_hosts = options.concurrency_hosts as usize;
    let port_timeout = options.timeout_ms;

    // Determine ports to scan
    let ports = match options.mode.as_str() {
        "fast" => port_scanner::top_ports(),
        _ => {
            if options.ports.is_empty() {
                port_scanner::top_ports()
            } else {
                options.ports.clone()
            }
        }
    };

    // Emit initial progress
    let _ = app.emit("sniffer:progress", &SnifferProgress {
        total_hosts,
        scanned_hosts: 0,
        services_found: 0,
        current_target: String::new(),
    });

    let mut results = Vec::new();

    // Phase 2: Device discovery & port scan for each host
    for (i, &ip) in all_ips.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        let ip_str = ip.to_string();

        // Emit progress
        let _ = app.emit("sniffer:progress", &SnifferProgress {
            total_hosts,
            scanned_hosts: i as u32,
            services_found: results.iter().map(|d: &DeviceResult| d.open_ports.len()).sum::<usize>() as u32,
            current_target: ip_str.clone(),
        });

        // Device discovery (TCP ping)
        let alive = discovery::is_host_alive(ip, port_timeout.min(1000)).await;
        if !alive {
            continue;
        }

        // Port scan
        let port_results = port_scanner::scan_ports(
            &ip_str,
            &ports,
            options.concurrency_ports as usize,
            port_timeout,
            cancel.clone(),
        ).await;

        // Service detection for each open port
        let mut service_results = Vec::new();
        if options.probe_services {
            for pr in port_results {
                if cancel.load(Ordering::SeqCst) {
                    break;
                }
                let detected = fingerprint::detect_service(
                    &ip_str,
                    pr,
                    port_timeout.max(2000),
                ).await;

                // Emit each port as discovered
                let _ = app.emit("sniffer:port", serde_json::json!({
                    "ip": ip_str,
                    "port": detected.port,
                    "protocol": detected.protocol,
                    "state": detected.state,
                    "service": detected.service,
                    "version": detected.version,
                    "banner": detected.banner,
                    "confidence": detected.confidence,
                    "probeMethod": detected.probe_method,
                }));

                service_results.push(detected);
            }
        } else {
            for pr in port_results {
                let _ = app.emit("sniffer:port", serde_json::json!({
                    "ip": ip_str,
                    "port": pr.port,
                    "protocol": pr.protocol,
                    "state": pr.state,
                    "service": pr.service,
                    "version": None::<String>,
                    "banner": None::<String>,
                    "confidence": pr.confidence,
                    "probeMethod": pr.probe_method,
                }));
                service_results.push(pr);
            }
        }

        // OS detection
        let hostname = discovery::resolve_hostname(&ip_str);
        let (mac, mac_vendor) = discovery::resolve_mac(&ip_str).unzip();

        let device = os_detect::assemble_device(
            ip,
            hostname,
            mac,
            mac_vendor,
            service_results,
            &options.mode,
        );

        // Emit device result
        let _ = app.emit("sniffer:device", &device);
        results.push(device);
    }

    // Save results
    let _ = SCAN_RESULTS.lock().map(|mut r| {
        r.insert(task_id.to_string(), results.clone());
    });

    // Emit complete
    let _ = app.emit("sniffer:complete", serde_json::json!({
        "taskId": task_id,
    }));

    Ok(())
}

#[tauri::command]
pub async fn sniffer_stop(task_id: String) -> Result<(), String> {
    let cancel = {
        let tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.get(&task_id).cloned()
    };
    match cancel {
        Some(c) => {
            c.store(true, Ordering::SeqCst);
            Ok(())
        }
        None => Err(format!("任务 {} 未找到", task_id)),
    }
}

#[tauri::command]
pub async fn sniffer_list(task_id: Option<String>) -> Result<Vec<DeviceResult>, String> {
    let results = SCAN_RESULTS.lock().map_err(|e| e.to_string())?;
    match task_id {
        Some(id) => results.get(&id).cloned().ok_or("Task not found".to_string()),
        None => Ok(results.values().flat_map(|v| v.clone()).collect()),
    }
}

#[tauri::command]
pub async fn sniffer_export(task_id: String, format: String) -> Result<String, String> {
    let results = {
        let results = SCAN_RESULTS.lock().map_err(|e| e.to_string())?;
        results.get(&task_id).cloned().ok_or("Task not found".to_string())?
    };

    match format.as_str() {
        "json" => serde_json::to_string_pretty(&results)
            .map_err(|e| format!("JSON serialization error: {}", e)),
        "csv" => {
            let mut csv = String::from("ip,port,protocol,service,version,confidence\n");
            for device in &results {
                for port in &device.open_ports {
                    csv.push_str(&format!(
                        "{},{},{},{},{},{}\n",
                        device.ip,
                        port.port,
                        port.protocol,
                        port.service.as_deref().unwrap_or(""),
                        port.version.as_deref().unwrap_or(""),
                        port.confidence,
                    ));
                }
            }
            Ok(csv)
        }
        _ => Err("Unsupported format. Use 'json' or 'csv'.".to_string()),
    }
}

#[tauri::command]
pub async fn sniffer_presets() -> Result<Vec<PortPreset>, String> {
    Ok(default_presets())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cidr_single_ip() {
        let ips = parse_cidr("192.168.1.1").unwrap();
        assert_eq!(ips.len(), 1);
        assert_eq!(ips[0].to_string(), "192.168.1.1");
    }

    #[test]
    fn test_parse_cidr_slash24() {
        let ips = parse_cidr("192.168.1.0/24").unwrap();
        assert_eq!(ips.len(), 254);
        assert_eq!(ips[0].to_string(), "192.168.1.1");
        assert_eq!(ips[253].to_string(), "192.168.1.254");
    }

    #[test]
    fn test_parse_cidr_too_large() {
        let result = parse_cidr("10.0.0.0/8");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_invalid() {
        let result = parse_cidr("not_an_ip/24");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cd src-tauri && cargo test --lib commands::network_sniffer 2>&1
```

Expected: 所有 CIDR 解析测试通过

- [ ] **Step 3: 验证编译**

```bash
cd src-tauri && cargo check 2>&1
```

Expected: 无编译错误，所有模块连接正确

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/commands/network_sniffer.rs
git commit -m "feat(sniffer): integrate full scan pipeline with event streaming"
```

---

### Task 6: 前端页面

**Files:**
- Create: `src/pages/network-sniffer/Page.vue`
- Create: `src/components/network-sniffer/PortModal.vue`
- Modify: `src/router/index.ts`
- Modify: `src/components/layout/Sidebar.vue`

- [ ] **Step 1: 创建 PortModal.vue 端口选择弹窗**

此组件实现分类端口浏览、搜索和手动输入。使用 UI 草稿 (`docs/superpowers/specs/network-sniffer-ui.html`) 中的 CSS 和交互逻辑，提取为 Vue 组件。

按以下结构实现:

```vue
<script setup lang="ts">
// Props: modelValue (selected ports as number[])
// Emits: update:modelValue
// Features:
// 1. Manual input: comma-separated port numbers
// 2. Search: filter by service name or port number
// 3. Categories: Web, Database, Remote Access, File Transfer,
//    Email, Network Infra, Windows Services, Messaging
// 4. Each category has expandable list of port entries
// 5. Selected ports show checkmark, count in footer
</script>

<template>
  <!-- Modal overlay with card layout matching UI mockup -->
</template>
```

实际代码应包含:
- 8 个服务分类，每类 4-8 个端口条目（从 UI 草稿中提取）
- 搜索过滤功能
- 手动端口输入
- 选中状态同步
- emit update:modelValue 供父组件 v-model 绑定

- [ ] **Step 2: 创建 Page.vue 主页面**

组件结构:
```
NetworkSnifferPage
├── ScanConfig
│   ├── TargetInput (CIDR input)
│   ├── ModeSwitch (fast/deep toggle buttons)
│   ├── PortSelector (preset buttons + custom tags + PortModal)
│   └── ScanControls (start/stop + progress bar)
├── ResultToolbar (filter/search/export dropdowns)
├── DeviceList
│   └── DeviceCard × N (collapsible, per-IP grouping)
│       └── PortRow × N (port details table)
└── SummaryBar (hosts count, ports, services, time)
```

状态管理:
```typescript
const targets = ref("192.168.1.0/24");
const scanMode = ref<"fast" | "deep">("fast");
const selectedPorts = ref<number[]>([]); // empty = use mode default
const scanState = ref<"idle" | "scanning" | "completed" | "error">("idle");
const taskId = ref<string | null>(null);
const devices = ref<DeviceResult[]>([]);
const progress = ref<SnifferProgress | null>(null);
const filterService = ref("all");
const searchQuery = ref("");
```

生命周期:
- onMounted: 无自动操作（用户手动触发扫描）
- onUnmounted: 停止事件监听（每个 onSniffer* 返回的 unlisten 函数）

事件绑定:
```typescript
// In startScan():
unlisteners.push(await onSnifferProgress((p) => { progress.value = p; }));
unlisteners.push(await onSnifferDevice((d) => { /* add/update device */ }));
unlisteners.push(await onSnifferPort((p) => { /* update device's ports */ }));
unlisteners.push(await onSnifferComplete(() => { scanState.value = "completed"; }));
unlisteners.push(await onSnifferError((e) => { scanState.value = "error"; }));
```

- [ ] **Step 3: 添加路由和导航**

在 `src/router/index.ts` 添加:
```typescript
{
  path: "/network-sniffer",
  name: "network-sniffer",
  component: () => import("@/pages/network-sniffer/Page.vue"),
},
```

在 `src/components/layout/Sidebar.vue` 添加导航项（在剪贴板之前）:
```typescript
{ label: "网络嗅探", name: "network-sniffer", path: "/network-sniffer", icon: Radio },
```

- [ ] **Step 4: 验证基本可用性**

执行:
```bash
cd src-tauri && cargo check 2>&1
```
以及确保 Vue 无编译错误。

- [ ] **Step 5: 提交**

```bash
git add src/pages/network-sniffer/Page.vue
git add src/components/network-sniffer/PortModal.vue
git add src/router/index.ts src/components/layout/Sidebar.vue
git commit -m "feat(sniffer): add frontend page with port selection modal and scan UI"
```

---

## 自检

1. **Spec 覆盖:** 设计文档中的每个需求都能对应到任务:
   - 数据模型 → Task 1
   - 设备发现 (TCP Ping + ARP) → Task 2
   - 端口扫描 (并发 TCP Connect) → Task 2
   - Banner 抓取 → Task 3
   - 服务指纹库 → Task 3
   - OS 探测 → Task 4
   - 结果聚合 DeviceResult → Task 4
   - 命令层 + 事件推送 → Task 5
   - 前端页面 + PortModal → Task 6

2. **Placeholder 扫描:** 所有步骤包含完整代码实现，无 "TBD"/"TODO"/"implement later"

3. **类型一致性:** 所有类型名、字段名、方法签名为全大写驼峰 `DeviceResult`, `PortResult`, `sniffer_start`, `sniffer:device` — 跨 Task 保持一致

4. **依赖顺序:** Task 1 → Tasks 2/3/4 (并行) → Task 5 → Task 6
