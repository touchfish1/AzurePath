# 网络嗅探功能设计文档

> **设计目标:** 在 AzurePath 中实现类 nmap 的局域网网络嗅探功能，自动发现在线设备、检测开放端口、识别运行服务及版本。
>
> **技术路线:** Rust 自实现 TCP 连接探测 + Banner 抓取 + 轻量级指纹库，纯 Rust 实现，无 nmap 外部依赖。

---

## 1. 架构总览

```
┌─────────────────────────────────────────────────────────┐
│                     Frontend (Vue)                       │
│  TargetInput │ ModeSwitch │ PortSelector │ ResultTable  │
└──────────────────────────┬──────────────────────────────┘
                           │ Tauri Commands
                           ▼
┌─────────────────────────────────────────────────────────┐
│              Tauri Command Layer (Rust)                  │
│  sniffer_start / sniffer_stop / sniffer_status          │
│  sniffer_results / sniffer_export                       │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                  Core Engine                             │
├─────────────────┬───────────────────┬───────────────────┤
│  Device Discovery│  Port Scanner     │  Service Detect   │
│  · ARP Scan      │  · SYN/TCP Conn   │  · Banner Grab    │
│  · Ping Sweep    │  · Concurrent     │  · HTTP Probe     │
│  · mDNS/DNS      │  · Port Presets   │  · TLS Handshake  │
│  · NetBIOS       │  · Custom Range   │  · Fingerprint DB │
├─────────────────┴───────────────────┴───────────────────┤
│                    Results Store                          │
│  HashMap<DeviceIp, DeviceResult> + event stream          │
└─────────────────────────────────────────────────────────┘
```

---

## 2. 核心数据模型

```rust
pub struct SnifferOptions {
    pub targets: Vec<String>,        // IP/CIDR 列表
    pub ports: Vec<u16>,             // 要扫描的端口
    pub mode: ScanMode,              // 快速 / 深度
    pub concurrency_hosts: u32,       // 并发主机数 (默认 10)
    pub concurrency_ports: u32,       // 每主机并发端口数 (默认 50)
    pub timeout_ms: u64,              // 连接超时 (快速 500ms, 深度 2000ms)
    pub probe_services: bool,         // 是否进行服务探测 (深度模式开启)
}

pub enum ScanMode {
    Fast,    // TOP 100 端口, 快速 banner 抓取
    Deep,    // 1-65535 或指定范围, 主动探测 + 指纹识别
}

pub struct DeviceResult {
    pub ip: String,
    pub hostname: Option<String>,
    pub mac: Option<String>,
    pub vendor: Option<String>,       // MAC 厂商
    pub os: Option<String>,           // OS 推测
    pub open_ports: Vec<PortResult>,
    pub is_alive: bool,
    pub scan_mode: String,            // "fast" | "deep"
    pub scan_completed: bool,
}

pub struct PortResult {
    pub port: u16,
    pub protocol: String,             // "tcp" | "udp"
    pub state: String,                // "open" | "filtered"
    pub service: Option<String>,      // 服务名 (如 "nginx")
    pub version: Option<String>,      // 版本号 (如 "1.24.0")
    pub banner: Option<String>,       // 原始 banner 响应
    pub confidence: u8,               // 置信度 0-100
    pub probe_method: String,         // "banner" | "http" | "tls" | "fingerprint"
}

pub struct SnifferProgress {
    pub total_hosts: u32,
    pub scanned_hosts: u32,
    pub total_ports: u32,
    pub scanned_ports: u32,
    pub services_found: u32,
    pub current_target: String,
}
```

---

## 3. 端口选择系统

### 预设分组

| 预设 | 端口范围 | 说明 |
|------|---------|------|
| 常用 TOP 100 | nmap top 100 端口 | 覆盖绝大多数常见服务 |
| 所有端口 | 1-65535 | 全端口扫描（深度模式） |
| 常见 Web | 80, 443, 8080, 8443, 3000, 5000, 8000, 9000 | Web 服务相关 |
| 数据库 | 3306, 5432, 1433, 1521, 27017, 6379, 9200, 11211 | 数据库端口集 |

### 自定义弹窗

弹窗包含三个组件：

1. **手动输入** — 文本框输入端口号（逗号或空格分隔），支持 `1-1000` 范围语法
2. **搜索过滤** — 按服务名（ssh, mysql）或端口号（3306）实时过滤
3. **分类列表** — 按服务类别组织，每种服务显示端口号 + 服务名，勾选加入端口列表

已选端口在主界面以彩色 tag 展示，每 tag 可单独移除。

---

## 4. 扫描流程

### 4.1 设备发现 (Device Discovery)

```
输入: CIDR 地址段 (e.g. 192.168.1.0/24)
  │
  ├── 并行发送 ICMP Ping
  ├── 并行执行 ARP 请求 (更准确，局域网内)
  ├── 可选: mDNS / NetBIOS 查询主机名
  │
  └── 存活设备列表 ──→ 进入端口扫描
```

- ARP 请求使用 `pnet` crate 发送原始以太网帧
- ICMP Ping 使用 `tokio::net::IcmpSocket`
- 超时未响应视为离线

### 4.2 端口扫描 (Port Scanning)

```
对每个存活设备:
  ┌─ 快速模式: 仅扫描 TOP 100 端口
  │             并发 50 个端口, 超时 500ms
  │             仅 TCP Connect (SYN → SYN-ACK → RST)
  │
  └─ 深度模式: 扫描用户指定端口或全端口
                并发 50 个端口, 超时 2000ms
                TCP Connect + Banner 抓取
```

实现方式:
- 使用 `tokio::net::TcpStream::connect_timeout` 进行 TCP 连接探测
- 并发控制使用 `tokio::semaphore` + `futures::stream::FuturesUnordered`
- 连接成功 = 端口开放
- 连接超时 + 无 RST = 可能 filtered（不做深度区分）
- 快速模式收到 SYN-ACK 后立即 RST 断开，节省资源

### 4.3 服务版本检测 (Service Version Detection)

```
对每个开放端口:
  ┌─ Banner 抓取:
  │   连接到端口, 读取 4KB 初始数据
  │   超时 2s (快速模式) / 5s (深度模式)
  │
  ├─ 主动探测 (深度模式):
  │   · 80/8080/8000: 发送 "GET / HTTP/1.0\r\n\r\n"
  │   · 443/8443: TLS 握手, 读取证书 Common Name
  │   · 21: FTP 默认 banner
  │   · 22: SSH banner
  │   · 25: SMTP EHLO
  │
  └─ 指纹匹配:
       将 banner 与指纹库匹配 → 服务名 + 版本号
       匹配算法: 正则 + 关键字评分
       置信度: 精确匹配 ≥ 90%, 正则匹配 ≥ 70%, 仅 banner 存在 ≥ 50%
```

### 4.4 OS 探测

快速模式:
- 分析 TTL 值: Windows (128), Linux (64), macOS (64), 路由器 (255)
- 结合开放端口组合判断

深度模式:
- TCP 初始窗口大小分析
- 结合 mDNS/NetBIOS 获取的主机名判断

---

## 5. 指纹库设计

轻量级内置指纹库，覆盖约 60-80 种常见服务。

```rust
pub struct ServiceFingerprint {
    pub port: u16,                    // 默认端口
    pub service_name: String,         // 服务名 (如 "nginx")
    pub transport: String,            // "tcp" | "udp"
    pub banner_patterns: Vec<BannerPattern>,
    pub probe_type: ProbeType,        // 探测方式
    pub probes: Vec<Probe>,           // 主动探测内容
}

pub struct BannerPattern {
    pub regex: String,                // 正则 (如 r"(?i)Server: nginx/([\d.]+)")
    pub confidence: u8,               // 匹配到此条的置信度
}

pub enum ProbeType {
    None,           // 仅读取初始 banner
    HttpGet,        // 发送 GET /
    TlsHandshake,   // TLS 握手
    SmtpEhlo,       // SMTP EHLO
    FtpAuth,        // FTP 匿名登录
}
```

覆盖范围（按类别）:

| 类别 | 服务 |
|------|------|
| Web 服务器 | nginx, Apache, IIS, Tomcat, Caddy, Lighttpd, Node.js |
| 数据库 | MySQL, PostgreSQL, MariaDB, Redis, MongoDB, Elasticsearch, MSSQL, Oracle |
| 远程访问 | OpenSSH, Dropbear, Telnet, RDP, VNC |
| 文件共享 | vsftpd, proftpd, Samba, NetBIOS |
| 邮件 | Postfix, Sendmail, Dovecot, Exim, Courier |
| 代理/网关 | Squid, HAProxy, Nginx, Envoy |
| 网络设备 | dnsmasq, UPnP/miniupnpd, NTP |
| 消息/缓存 | ZooKeeper, Kafka, ActiveMQ, Memcached |
| 其他 | OpenVPN, Docker API, Kubernetes API |

---

## 6. 前端组件设计

### 页面布局 (Page.vue)

```
┌──────────────────────────────────────────────┐
│  网络嗅探                                      │
│  扫描局域网设备，检测开放端口与运行服务           │
├──────────────────────────────────────────────┤
│  [目标输入: 192.168.1.0/24]                   │
│  [快速扫描▸  TOP 100]  [深度扫描▸  全端口]     │
│  [开始扫描] [停止]                             │
├──────────────────────────────────────────────┤
│  预设: [常用] [所有] [Web] [数据库]             │
│  端口: [80] [443] [22] [3306] [+ 自定义]      │
│  ████████████████░░░░ 67% · 47/70 主机       │
├──────────────────────────────────────────────┤
│  扫描结果                    筛选: [全部▼] 🔍 │
│                                              │
│  ┌─ 192.168.1.1  router.local               │
│  │  22/tcp  SSH      OpenSSH 8.9p1    ██████ │
│  │  80/tcp  HTTP     nginx 1.24.0     ██████ │
│  │  443/tcp HTTPS    nginx 1.24.0     ██████ │
│  └──                                        │
│  ┌─ 192.168.1.42  nas-home.local            │
│  │  3306/tcp MySQL   8.0.35          ██████  │
│  │  6379/tcp Redis   7.2.4           ██████  │
│  └──                                        │
│                                              │
│  3 台在线 · 16 端口 · 10 服务 · 47s          │
└──────────────────────────────────────────────┘
```

### 组件树

```
NetworkSnifferPage
├── ScanConfig
│   ├── TargetInput        (CIDR 输入 + 验证)
│   ├── ModeSwitch         (快速/深度切换)
│   ├── PortSelector       (预设按钮 + 自定义弹窗)
│   │   └── PortModal      (分类端口选择弹窗)
│   └── ScanControls       (开始/停止按钮 + 进度条)
├── ResultToolbar          (筛选/搜索/导出)
├── DeviceList
│   └── DeviceCard × N     (按 IP 分组)
│       └── PortRow × N    (端口详情行)
└── SummaryBar             (统计汇总)
```

### 状态管理

```
- targets: string[]        // 输入框多个目标
- scanMode: 'fast' | 'deep'
- selectedPorts: number[]  // 用户选择的端口
- scanState: 'idle' | 'scanning' | 'completed' | 'error'
- progress: SnifferProgress
- devices: DeviceResult[]  // 实时增量更新
- filter: { service: string, search: string }
```

---

## 7. Tauri 命令设计

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `sniffer_start` | options: SnifferOptions | task_id: String | 开始异步扫描 |
| `sniffer_stop` | task_id: String | void | 停止扫描 |
| `sniffer_list` | - | DeviceResult[] | 获取当前结果 |
| `sniffer_export` | format: "json"\|"csv" | String | 导出结果 |
| `sniffer_presets` | - | PortPreset[] | 获取预设列表 |

### 事件

| 事件 | 载荷 | 频率 |
|------|------|------|
| `sniffer:progress` | SnifferProgress | 每完成一台主机 |
| `sniffer:device` | DeviceResult | 每台设备扫描完成 |
| `sniffer:port` | PortResult | 每个端口发现 |
| `sniffer:complete` | { task_id, summary } | 全部完成 |
| `sniffer:error` | { task_id, error } | 出错时 |

---

## 8. 数据流

```
用户点击 "开始扫描"
  → invoke sniffer_start(options)
  → Tauri 后端创建扫描任务
  → 异步 Spawn 引擎:
     1. 解析 CIDR → IP 列表
     2. 设备发现 (ARP + ICMP)
        → emit sniffer:progress (更新存活主机)
     3. 对每台存活设备:
        a. 并行端口探测 (TCP Connect)
           → emit sniffer:port (实时流式更新)
        b.  Banner 抓取 + 指纹匹配
        c.  聚合为 DeviceResult
           → emit sniffer:device
     4. emit sniffer:complete
  → 前端监听事件，实时更新结果表格
  → 用户可随时 sniffer_stop 终止
```

前端接收到 `sniffer:port` 事件时即更新对应设备的端口列表行，实现"边扫边出"效果。

---

## 9. 并发与资源控制

| 参数 | 快速模式 | 深度模式 |
|------|---------|---------|
| 并发主机数 | 20 | 10 |
| 每主机并发端口 | 50 | 50 |
| 连接超时 | 500ms | 2000ms |
| Banner 读取超时 | 2s | 5s |
| TOP N 端口 | 100 | 用户指定或全端口 |

- 使用 `tokio::semaphore` 控制全局并发
- 支持用户配置并发数和超时
- 扫描中可随时取消（通过 CancelToken pattern）

---

## 10. 错误处理

- CIDR 格式校验：前端 `ip-cidr.js` 库验证，后端 `ipnetwork` crate 解析
- 无权限（原始 socket）：降级为 TCP Connect 模式（纯用户态，无需管理员权限）
- 扫描中断：已收集的结果不丢失，前端保留至下次扫描
- 空目标：提示输入有效 CIDR
- 防火墙拦截：端口标记为 filtered，不影响其他端口

---

## 11. 实现计划

### Task 1: 数据模型 + Tauri 命令骨架
- 创建 `src-tauri/src/types/network_sniffer.rs`
- 创建 `src-tauri/src/commands/network_sniffer.rs`
- 注册命令到 `lib.rs`
- 输出：可调通的空命令

### Task 2: 设备发现模块
- ARP 扫描 + ICMP Ping 实现
- 并发设备发现
- 输出：可发现局域网存活设备

### Task 3: 端口扫描模块
- TCP Connect 扫描器
- 并发控制 + 预设端口组
- 事件通知
- 输出：可扫描指定主机的端口

### Task 4: Banner 抓取 + 服务指纹库
- Banner 读取
- HTTP 主动探测
- TLS 握手探测
- 内置指纹库 (60-80 服务)
- 输出：可识别服务版本

### Task 5: OS 探测 + 结果聚合
- TTL 分析
- 端口组合分析
- DeviceResult 组装
- 输出：完整的扫描结果

### Task 6: 前端页面
- Page.vue 完整实现
- PortModal 组件
- 设备列表 + 端口详情
- 进度条 + 汇总

### Task 7: 扫描引擎集成
- 命令层串联完整流程
- CIDR 解析 → 设备发现 → 端口扫描 → 服务识别
- 实时事件推送
