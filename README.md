# AzurePath

[![CI](https://github.com/chengccn/AzurePath/actions/workflows/ci.yml/badge.svg)](https://github.com/chengccn/AzurePath/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Tauri](https://img.shields.io/badge/Tauri-2.0-purple)
![Vue](https://img.shields.io/badge/Vue-3.5-brightgreen)
![License](https://img.shields.io/badge/license-MIT-blue)

**AzurePath** 是一款跨平台桌面端内网运维工具箱，基于 Tauri 2.0 构建。集网络诊断、文件传输、聊天通信、剪贴板管理、设备发现等工具于一体。

AzurePath is a cross-platform desktop intranet operations toolbox built with Tauri 2.0. It integrates network diagnostics, file transfer, chat, clipboard management, device discovery, and more.

---

## 功能 Features

### 🔧 网络诊断 Network Diagnostics
- **📡 Ping** — ICMP ping 检测，支持自定义次数、间隔、超时，跨平台中英文解析 / Customizable count, interval, timeout, cross-platform bilingual parsing
- **🗺️ Traceroute** — 路由追踪，逐跳探测途经节点 / Per-hop route tracing with Windows/Unix output parsing
- **🔍 端口扫描 Port Scan** — TCP 并发端口扫描，支持自定义范围和并发数 / Concurrent TCP port scanning
- **🌐 DNS 查询** — 支持 A/AAAA/CNAME/MX/NS/SOA/TXT/ALL 记录，自定义 DNS 服务器 / DNS record lookup with custom DNS servers
- **🔄 MTR 路由追踪** — 结合 Traceroute + Ping，实时显示每跳的延迟、丢包率、抖动统计 / Combines traceroute and ping for real-time per-hop latency, loss, and jitter statistics
- **📡 网络嗅探 Network Sniffer** — 局域网设备发现、端口扫描、服务 Banner 抓取、指纹识别、OS 探测、并发主机扫描 / LAN device discovery, port scanning, banner grabbing, OS fingerprinting, concurrent host scanning

### 💬 通信与协作 Communication & Collaboration
- **💬 LAN 聊天** — 局域网设备发现 + 点对点即时消息，带系统通知提醒 / LAN peer discovery + peer-to-peer instant messaging with system notifications
- **📁 文件传输 File Transfer** — 局域网内点对点文件直传，支持拖拽发送 / LAN peer-to-peer file transfer with drag-and-drop support
- **📋 剪贴板管理 Clipboard Manager** — 剪贴板历史记录、持久化存储、搜索、收藏、图片预览 / Clipboard history with persistence, search, favorites, and image preview

### 🧰 工具箱 Toolbox
- **子网计算器 Subnet Calculator** — IPv4/IPv6 CIDR 计算、子网划分、IP 地址分类（私有/公网/环回/链路本地/多播）/ IPv4/IPv6 CIDR calculation, subnet splitting, IP classification (private/public/loopback/link-local/multicast)
- **Base64 编解码** / Base64 encode/decode
- **URL 编解码** / URL encode/decode
- **Hash 生成器** — MD5/SHA1/SHA256/SHA512
- **端口速查** — 常用端口号与服务对应查询 / Common port number lookup
- **WiFi QR 生成器** — 生成 WiFi 连接二维码 / WiFi QR code generator
- **JSON 格式化** / JSON formatter & validator
- **JWT 解码器** / JWT decoder
- **时间戳转换** / Unix timestamp converter

### 📊 仪表盘与监控 Dashboard & Monitoring
- **活动概览 Activity Overview** — 首页聚合展示近期动态 / Home page activity feed
- **历史记录 History** — 全部活动/收藏/时间线三种视图 / Activity history with favorites and timeline views
- **性能历史 Performance History** — 网络延迟、带宽使用趋势图 / Network latency and bandwidth usage trends
- **带宽监控 Bandwidth Monitor** — 实时网速监控 / Real-time network interface bandwidth monitoring
- **网络监控 Network Monitor** — 目标可达性监控，带历史记录 / Target reachability monitoring with history

### ⚙️ 系统功能 System Features
- **系统托盘 System Tray** — 最小化到托盘后台运行 / Minimize to system tray
- **系统通知 Notifications** — 文件传输、新消息、扫描完成推送 OS 通知 / File transfer, new messages, scan completion notifications
- **全局快捷键 Global Shortcut** — `Ctrl+Alt+A` 唤出窗口 / Show window
- **快捷键导航 Shortcuts** — `Ctrl+1~9` 切换页面，`Ctrl+T` 切换主题 / Page navigation, theme toggle
- **主题切换 Theme** — 亮色/暗色/跟随系统 / Light/dark/system theme with scheduled switching
- **命令面板 Command Palette** — `Ctrl+K` 快速搜索和导航 / Quick search and navigation
- **书签系统 Bookmarks** — 常用页面和工具收藏 / Bookmark frequently used pages and tools
- **目标分组 Target Groups** — 管理常用网络目标 / Organize network targets into groups
- **自动更新 Auto Updater** — tauri-plugin-updater 支持 / Automatic updates

---

## 技术栈 Tech Stack

| 层 Layer | 技术 Technology |
|----------|-----------------|
| 桌面框架 Desktop Framework | Tauri 2.0 |
| 后端语言 Backend | Rust (tokio async runtime) |
| 前端框架 Frontend | Vue 3 + TypeScript + Composition API |
| UI 组件 UI | Tailwind CSS v4 |
| 状态管理 State | Pinia |
| 路由 Router | Vue Router |
| 数据库 Database | SQLite (rusqlite, bundled) |
| 日志 Logging | tracing + tracing-subscriber |
| 测试 Testing | Rust: built-in test harness / Frontend: vitest + @vue/test-utils |

---

## 快速开始 Quick Start

### 环境要求 Prerequisites

- [Rust](https://www.rust-lang.org/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- [Tauri 2.0 系统依赖](https://v2.tauri.app/start/prerequisites/)

### 启动开发服务器 Start Dev Server

```bash
# 安装前端依赖 Install dependencies
npm install

# 启动 Tauri 开发模式（前端 + 后端热重载）
# Start Tauri dev mode (frontend + backend hot-reload)
npm run tauri dev
```

### 构建 Build

```bash
npm run tauri build
```

### 测试 Testing

```bash
# Rust 后端测试 Backend tests
cd src-tauri && cargo test

# Rust 代码检查 Type check
cd src-tauri && cargo check

# 前端类型检查 Frontend type check
npx vue-tsc --noEmit

# 前端测试 Frontend tests
npm test
```

---

## 键盘快捷键 Keyboard Shortcuts

| 快捷键 Shortcut | 功能 Action |
|-----------------|-------------|
| `Ctrl+1` ~ `Ctrl+9` | 切换页面导航 Page navigation |
| `Ctrl+K` | 命令面板 Command palette |
| `Ctrl+T` | 切换亮色/暗色主题 Toggle theme |
| `Ctrl+D` | 跳转到仪表盘 Go to dashboard |
| `Ctrl+F` | 跳转到文件传输 Go to file transfer |
| `Ctrl+Alt+A` | 全局唤出窗口 Show window (global) |
| `Escape` | 关闭弹窗/取消操作 Close modal / cancel |

---

## 架构 Architecture

项目采用经典三层架构 Three-layer architecture:

```
src/                          # 前端源码 Frontend (Vue 3 + TypeScript)
├── components/               # 通用 UI 组件
│   ├── layout/               # AppShell, TitleBar, Sidebar
│   └── ui/                   # Button, Toast, etc.
├── composables/              # 可复用组合式函数 Reusable composables
├── pages/                    # 页面组件 Pages
│   ├── dashboard/            # 仪表盘 Dashboard
│   ├── ping/                 # Ping
│   ├── traceroute/           # 路由追踪 Traceroute
│   ├── mtr/                  # MTR 路由追踪 MTR
│   ├── port-scan/            # 端口扫描 Port Scan
│   ├── dns/                  # DNS 查询 DNS Lookup
│   ├── chat/                 # LAN 聊天 LAN Chat
│   ├── clipboard/            # 剪贴板管理 Clipboard Manager
│   ├── files/                # 文件传输 File Transfer
│   ├── network-sniffer/      # 网络嗅探 Network Sniffer
│   ├── history/              # 活动历史 Activity History
│   ├── toolbox/              # 工具箱 Toolbox
│   ├── speedtest/            # 网速测试 Speedtest
│   ├── topology/             # 网络拓扑 Network Topology
│   ├── monitor/              # 网络监控 Network Monitor
│   ├── bandwidth/            # 带宽监控 Bandwidth Monitor
│   ├── mdns/                 # mDNS 发现 mDNS Discovery
│   ├── api-test/             # API 测试 API Test
│   ├── bookmarks/            # 书签 Bookmarks
│   ├── target-groups/        # 目标分组 Target Groups
│   ├── wol/                  # 远程唤醒 Wake-on-LAN
│   ├── backup/               # 备份与恢复 Backup & Restore
│   ├── logs/                 # 日志查看 Log Viewer
│   └── settings/             # 设置 Settings
├── lib/                      # 工具函数 Utilities
│   ├── tauri.ts              # Tauri invoke/event 封装
│   └── format.ts             # 格式化工具 Formatting helpers
├── router/                   # 路由配置 Router config
└── stores/                   # Pinia 状态管理 Stores

src-tauri/                    # 后端源码 Backend (Rust)
├── src/
│   ├── commands/             # #[tauri::command] 命令层
│   │   ├── ping.rs           # Ping 命令
│   │   ├── mtr.rs            # MTR 命令
│   │   ├── subnet.rs         # 子网计算命令
│   │   ├── network_sniffer.rs # 网络嗅探命令
│   │   └── ...               # 更多命令
│   ├── core/                 # 核心业务逻辑 Business Logic
│   │   ├── ping/             # Ping 引擎
│   │   ├── mtr/              # MTR 引擎
│   │   ├── subnet/           # 子网计算引擎
│   │   ├── cancel.rs         # 共享取消令牌 Shared Cancel Token
│   │   ├── traceroute/       # 路由追踪引擎
│   │   ├── port_scan/        # 端口扫描引擎
│   │   ├── dns/              # DNS 解析器
│   │   ├── network_sniffer/  # 网络嗅探引擎
│   │   ├── chat/             # 聊天和消息持久化
│   │   ├── clipboard/        # 剪贴板监控和存储
│   │   ├── connection/       # LAN 连接管理
│   │   ├── discovery/        # 局域网设备发现
│   │   ├── file_transfer/    # 文件传输引擎
│   │   ├── file_server/      # HTTP 文件下载服务
│   │   ├── monitor/          # 网络监控引擎
│   │   ├── bandwidth/        # 带宽监控引擎
│   │   ├── mdns/             # mDNS 发现
│   │   ├── utils.rs          # 共享工具函数
│   │   └── settings.rs       # 设置持久化
│   └── types/                # 数据模型（Serialize + Deserialize）
├── capabilities/             # Tauri v2 权限声明
└── Cargo.toml

docs/                         # 设计文档和计划
└── superpowers/
    └── specs/                # 功能设计文档
```

### 三层职责 Layer Responsibilities

1. **`types/`** — 可序列化数据模型，`#[serde(rename_all = "camelCase")]` 保证前后端命名一致 / Serializable data models for TypeScript interop
2. **`core/`** — 纯业务逻辑层，不依赖 Tauri API，可独立测试 / Pure business logic, Tauri-independent, testable in isolation
3. **`commands/`** — `#[tauri::command]` 包装器，负责参数校验、调用 core、发送 Tauri 事件 / Tauri command wrappers for validation, orchestration, and event emission

### 设计模式 Design Patterns

- **取消模式 Cancellation**: 共享 `CancelRegistry`（`core/cancel.rs`），所有长任务统一取消机制 / Unified cancellation via shared `CancelRegistry`
- **事件驱动 Event-Driven**: 异步命令通过 Tauri events (`app.emit()`) 向前端推送进度 / Async commands stream progress via Tauri events
- **模块隔离 Modularity**: 每功能独立 types/core/commands 三层，松耦合 / Each feature has independent types/core/commands layers

---

## 许可证 License

MIT
