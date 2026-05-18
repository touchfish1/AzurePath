# AzurePath 内网运维工具箱 — 设计文档

## 概述

AzurePath 是一个跨平台桌面端内网运维工具箱，第一期聚焦网络诊断功能，后续扩展为覆盖资产管理、文件传输、运维面板、即时通讯的综合平台。

## 技术栈

| 层 | 技术 | 选型理由 |
|------|------|----------|
| 桌面框架 | **Tauri 2.0** | 性能优先，小体积，跨平台，安全沙箱 |
| 后端语言 | **Rust** | 零成本抽象，原生网络能力（ICMP/raw socket/异步 I/O） |
| 前端框架 | **Vue 3 + TypeScript** | 国内生态成熟，组合式 API 开发效率高 |
| UI 体系 | **shadcn-vue + Tailwind CSS** | 现代风格，编译时样式无运行时开销，定制灵活 |
| 状态管理 | **Pinia** | Vue 3 官方推荐，TypeScript 友好 |
| 异步运行时 | **tokio** | Rust 异步网络栈标准选择 |

## 架构设计

### 整体架构

```
┌──────────────────────────────────────────────────────────────┐
│  Tauri Desktop App                                           │
│                                                              │
│  ┌──────────────────────┐  ┌──────────────────────────────┐  │
│  │  Frontend (Vue 3)    │  │  Backend (Rust)               │  │
│  │                      │  │                               │  │
│  │  shadcn-vue UI       │◄─┤  Tauri IPC (invoke/event)     │  │
│  │  - 仪表盘            │  │  - 命令层 (commands/)          │  │
│  │  - Ping              │  │  - 网络引擎 (core/)            │  │
│  │  - Traceroute        │  │  - 数据模型 (types/)           │  │
│  │  - 端口扫描          │  │                               │  │
│  │  - DNS 查询          │  │  Rust 性能保障:               │  │
│  │                      │  │  - 异步非阻塞 I/O             │  │
│  │  Pinia 状态管理      │  │  - 并发控制                   │  │
│  │  Vue Router 页面     │  │  - 取消令牌                   │  │
│  │  Tailwind CSS 样式   │  │  - 零拷贝序列化               │  │
│  └──────────────────────┘  └──────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 设计原则

- **安全分层** — `core/` 层纯网络逻辑，不依赖 Tauri；`commands/` 层薄封装负责任务生命周期管理
- **流式推送** — 所有长时间运行的操作（ping、traceroute、端口扫描）通过 Tauri event 逐条推送结果，前端增量渲染
- **可取消** — 每个操作绑定 CancellationToken，用户取消时即时收尾
- **平台抽象** — 底层网络操作通过条件编译适配 Linux (AF_PACKET)、Windows (Npcap/WinDivert)、macOS (BPF)

## 项目结构

```
azurepath/
├── src/                          # 前端源码 (Vue 3 + TS)
│   ├── pages/
│   │   ├── index.vue             # 仪表盘
│   │   ├── ping/index.vue
│   │   ├── traceroute/index.vue
│   │   ├── port-scan/index.vue
│   │   ├── dns/index.vue
│   │   └── history/index.vue
│   ├── components/
│   │   ├── layout/
│   │   │   ├── AppSidebar.vue
│   │   │   └── AppHeader.vue
│   │   ├── network/
│   │   │   ├── PingResult.vue
│   │   │   ├── TracerouteResult.vue
│   │   │   ├── PortList.vue
│   │   │   └── DnsResult.vue
│   │   └── common/
│   │       ├── TerminalOutput.vue
│   │       └── TargetInput.vue
│   ├── lib/
│   │   └── tauri.ts              # IPC 封装层
│   ├── stores/
│   │   ├── ping.ts
│   │   ├── traceroute.ts
│   │   ├── port-scan.ts
│   │   └── dns.ts
│   └── router/
│       └── index.ts
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── ping.rs
│   │   │   ├── traceroute.rs
│   │   │   ├── port_scan.rs
│   │   │   └── dns.rs
│   │   ├── core/
│   │   │   ├── mod.rs
│   │   │   ├── ping/
│   │   │   │   ├── mod.rs
│   │   │   │   └── icmp.rs
│   │   │   ├── traceroute/
│   │   │   │   ├── mod.rs
│   │   │   │   └── raw_socket.rs
│   │   │   ├── port_scan/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── scanner.rs
│   │   │   │   └── tcp_connect.rs
│   │   │   └── dns/
│   │   │       ├── mod.rs
│   │   │       └── resolver.rs
│   │   └── types/
│   │       ├── mod.rs
│   │       ├── ping.rs
│   │       ├── traceroute.rs
│   │       ├── port_scan.rs
│   │       └── dns.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
├── tsconfig.json
├── vite.config.ts
└── tailwind.config.ts
```

## 网络工具设计

### 1. Ping 探测

| 维度 | 设计 |
|------|------|
| 实现 | Rust ICMP socket (raw socket)，跨平台条件编译 |
| 并发 | tokio 并发池同时探测多目标 |
| 超时 | tokio::time::timeout，精度 1ms |
| 数据流 | 每个 ICMP reply 通过 Tauri event 流式推送 |
| 取消 | CancellationToken 即时停止所有 in-flight 请求 |
| 统计 | Rust 端维护丢包率/延迟/抖动统计，请求结束时一次推送 |

### 2. Traceroute 路由追踪

| 维度 | 设计 |
|------|------|
| 实现 | 原生 raw socket 发送 ICMP echo 递增 TTL |
| 并发探测 | 可配置同时发出多个 TTL 探测包 |
| 超时 | 每跳独立超时，超时记为 timeout |
| 展示 | 前端按跳数排列，显示每个 hop 的 IP、域名、延迟 |

### 3. 端口扫描

| 维度 | 设计 |
|------|------|
| 实现 | TCP Connect 扫描 (tokio::net::TcpStream::connect) |
| 并发控制 | tokio Semaphore 控制最大并发连接数 |
| 范围 | 单端口、端口段 (1-1024)、常见端口列表 |
| 增量推送 | 扫描到开放端口即时推送，前端虚拟滚动展示 |
| 服务识别 | 匹配端口号与常见服务映射表 (SSH=22, HTTP=80 等) |

### 4. DNS 查询

| 维度 | 设计 |
|------|------|
| 实现 | trust-dns-proto / hickory-resolver (纯 Rust DNS) |
| 记录类型 | A, AAAA, CNAME, MX, NS, SOA, TXT |
| 并发 | 多记录类型并行查询 |
| 自定义 DNS | 支持指定 DNS 服务器地址 |

## Tauri IPC 接口

### 命令 (invoke)

```typescript
// Ping
'ping:start'   (target: string, options: PingOptions)  => void
'ping:stop'    (taskId: string)                         => void

// Traceroute
'traceroute:start' (target: string, options: TraceOptions) => void
'traceroute:stop'  (taskId: string)                        => void

// Port Scan
'port-scan:start' (target: string, ports: PortRange) => void
'port-scan:stop'  (taskId: string)                   => void

// DNS
'dns:lookup' (target: string, recordType: RecordType) => DnsResult
```

### 事件 (event)

```typescript
// Ping
'ping:progress' => { taskId, seq, ttl, latency, status: "success"|"timeout" }
'ping:complete' => { taskId, sent, received, loss%, min, avg, max, mdev }

// Traceroute
'trace:hop'      => { taskId, hop, addr, hostname?, latencies: number[] }
'trace:complete' => { taskId, hops: Hop[] }

// Port Scan
'port:progress' => { taskId, scanned, total, open }
'port:found'    => { taskId, port, service? }
'port:complete' => { taskId, openPorts: OpenPort[] }

// DNS
'dns:result' => { taskId, records: DnsRecord[] }
'dns:error'  => { taskId, message }
```

## 性能目标与策略

### 目标

| 指标 | 目标值 |
|------|--------|
| 应用冷启动 | < 1s |
| 常驻内存 | < 80 MB |
| 安装包体积 | < 15 MB |
| CJS 编译产物 | < 10 MB |
| 全端口扫描 (65535) | < 60s |
| Ping 100 目标并发 | 5s 内完成，CPU < 5% |

### 前端性能保障

- **shadcn-vue + Tailwind CSS** — 编译时样式，零运行时 CSS 开销
- **虚拟滚动** — 端口列表等大量数据使用虚拟滚动，DOM 节点常驻几十个
- **流式增量渲染** — ping/traceroute 逐行推送，前端追加，不阻塞 UI
- **操作竞态处理** — 用户重复点击时自动 abort 前次操作
- **按需加载** — 页面级懒加载，首屏只加载仪表盘

### Rust 后端性能保障

- **异步非阻塞** — tokio 运行时，零线程上下文切换开销
- **零拷贝 IPC** — Tauri IPC 使用 serde 序列化，指针传递
- **并发控制** — Semaphore 管理连接/任务并发量
- **取消机制** — CancellationToken 链，用户取消即时停掉所有 in-flight 任务

## 与 Electron 方案的性能对比

| 场景 | Tauri + Rust | Electron + Node.js |
|------|-------------|-------------------|
| Ping 100 目标并发 | 3-5s, CPU < 5% | 8-15s (child_process 开销), CPU 15-30% |
| 全端口扫描 | 30-60s | 120s+ |
| 常驻内存 | 50-80 MB | 200-400 MB |
| 冷启动 | < 1s | 2-5s |
| 安装包 | ~10 MB | ~150 MB |

核心差异：Rust 原生 raw socket / ICMP 能力避免了 Electron 必须 child_process 调系统命令的频繁进程创建开销和文本解析的不稳定性。

## 后续扩展性

当前架构设计为平台化预留了清晰的扩展路径：

| 模块 | Rust 侧 | 前端侧 |
|------|---------|--------|
| 文件传输 | tokio 文件 I/O + TCP 直连 | 新页面 + 进度条组件 |
| 资产管理 | 异步扫描 + 持久化 (SQLite) | 表格 + 筛选 + 趋势图 |
| 运维面板 | SSH 连接池 + 并发命令执行 | Dashboard 卡片布局 |
| 即时通讯 | WebSocket/TCP 长连接 | 聊天组件 + 通知系统 |

每新增一个模块，在 `commands/` 和 `core/` 下新增对应目录，前端新增页面和 store，不影响现有模块。

---

*设计版本 v1.0 — 2026-05-18*
