# AzurePath

[![CI](https://github.com/chengccn/AzurePath/actions/workflows/ci.yml/badge.svg)](https://github.com/chengccn/AzurePath/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Tauri](https://img.shields.io/badge/Tauri-2.0-purple)
![Vue](https://img.shields.io/badge/Vue-3.5-brightgreen)
![License](https://img.shields.io/badge/license-MIT-blue)

**AzurePath** 是一款跨平台桌面端内网运维工具箱，基于 Tauri 2.0 构建。集网络诊断、文件传输、聊天通信、剪贴板管理、设备发现等工具于一体。

---

## 功能

### 🔧 网络诊断
- **📡 Ping** — ICMP ping 检测，支持自定义次数、间隔、超时，跨平台中英文解析
- **🗺️ Traceroute** — 路由追踪，逐跳探测途经节点，支持 Windows/Unix 输出解析
- **🔍 端口扫描** — TCP 并发端口扫描，支持自定义范围和并发数
- **🌐 DNS 查询** — 支持 A / AAAA / CNAME / MX / NS / SOA / TXT / ALL 记录类型，支持自定义 DNS 服务器
- **📡 网络嗅探** — 局域网设备发现、端口扫描、服务 Banner 抓取、指纹识别、OS 探测、并发主机扫描

### 💬 通信与协作
- **💬 LAN 聊天** — 局域网设备发现 + 点对点即时消息，带系统通知提醒
- **📁 文件传输** — 局域网内点对点文件直传，独立文件传输管理页面，支持拖拽发送
- **📋 剪贴板管理** — 剪贴板历史记录、持久化存储、搜索、收藏、图片缩略图预览

### 🧰 工具箱
- **子网计算器** — IP/CIDR/子网掩码换算
- **Base64 编解码** — 文本 Base64 互转
- **URL 编解码** — URL 百分比编码/解码
- **Hash 生成器** — 支持 MD5/SHA1/SHA256/SHA512
- **端口速查** — 常用端口号与服务对应查询

### 📊 仪表盘
- **活动概览** — 首页聚合展示剪贴板、设备发现、活跃节点等近期动态
- **历史记录** — 支持全部活动/收藏/时间线三种视图，搜索过滤和批量操作

### ⚙️ 系统功能
- **系统托盘** — 最小化到托盘后台运行
- **系统通知** — 文件传输完成、新消息、扫描完成推送 OS 通知
- **全局快捷键** — `Ctrl+Alt+A` 唤出窗口
- **快捷键导航** — `Ctrl+1~9` 切换页面，`Ctrl+T` 切换主题
- **主题切换** — 亮色/暗色/跟随系统，持久化偏好
- **自动更新** — tauri-plugin-updater 支持

---

## 技术栈

| 层 | 技术 |
|------|----------|
| 桌面框架 | Tauri 2.0 |
| 后端语言 | Rust (tokio 异步运行时) |
| 前端框架 | Vue 3 + TypeScript + Composition API |
| UI 组件 | shadcn-vue + Tailwind CSS v4 |
| 状态管理 | Pinia |
| 路由 | Vue Router |
| 数据库 | SQLite (rusqlite, bundled) |
| 日志 | tracing + tracing-subscriber |
| 测试 | Rust: built-in test harness / 前端: vitest + @vue/test-utils |

---

## 快速开始

### 环境要求

- [Rust](https://www.rust-lang.org/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- [Tauri 2.0 系统依赖](https://v2.tauri.app/start/prerequisites/)

### 启动开发服务器

```bash
# 安装前端依赖
npm install

# 启动 Tauri 开发模式（前端 + 后端热重载）
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

### 测试

```bash
# 前端测试（52 个测试用例）
npm test

# Rust 后端测试
cd src-tauri && cargo test

# Rust 代码检查
cd src-tauri && cargo check

# 前端类型检查
npx vue-tsc --noEmit
```

---

## 键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+1` ~ `Ctrl+9` | 切换页面导航 |
| `Ctrl+T` | 切换亮色/暗色主题 |
| `Ctrl+D` | 跳转到仪表盘 |
| `Ctrl+F` | 跳转到文件传输 |
| `Ctrl+Alt+A` | 全局唤出窗口（应用最小化时） |
| `Escape` | 关闭弹窗/取消操作 |

---

## 架构

项目采用经典三层架构：

```
src/                          # 前端源码 (Vue 3 + TypeScript)
├── components/               # 通用 UI 组件（Button、Toast、Sidebar 等）
│   ├── layout/               # 布局组件（AppShell、TitleBar、Sidebar）
│   └── ui/                   # 基础 UI 组件
├── composables/              # 可复用组合式函数
├── pages/                    # 页面组件（每个功能一个页面）
│   ├── dashboard/            # 仪表盘
│   ├── ping/                 # Ping
│   ├── traceroute/           # 路由追踪
│   ├── port-scan/            # 端口扫描
│   ├── dns/                  # DNS 查询
│   ├── chat/                 # LAN 聊天
│   ├── clipboard/            # 剪贴板管理
│   ├── files/                # 文件传输
│   ├── network-sniffer/      # 网络嗅探
│   ├── history/              # 活动历史
│   └── toolbox/              # 工具箱
├── lib/                      # 工具函数和 Tauri 绑定
│   ├── tauri.ts              # Tauri invoke/event 封装
│   └── format.ts             # 格式化工具
├── router/                   # 路由配置
└── stores/                   # Pinia 状态管理

src-tauri/                    # 后端源码 (Rust)
├── src/
│   ├── commands/             # #[tauri::command] 命令层
│   ├── core/                 # 核心业务逻辑
│   │   ├── ping/             # Ping 引擎
│   │   ├── traceroute/       # 路由追踪引擎
│   │   ├── port_scan/        # 端口扫描引擎
│   │   ├── dns/              # DNS 解析器
│   │   ├── chat/             # 聊天和消息持久化
│   │   ├── clipboard/        # 剪贴板监控和存储
│   │   ├── connection/       # LAN 连接管理
│   │   ├── discovery/        # 局域网设备发现
│   │   ├── file_transfer/    # 文件传输引擎
│   │   ├── file_server/      # HTTP 文件下载服务
│   │   ├── network_sniffer/  # 网络嗅探引擎
│   │   ├── utils.rs          # 共享工具函数
│   │   └── settings.rs       # 设置持久化
│   └── types/                # 数据模型（Serialize + Deserialize）
├── capabilities/             # Tauri v2 权限声明
└── Cargo.toml

docs/                         # 设计文档和计划
```

### 三层职责

1. **`types/`** — 可序列化数据模型，`#[serde(rename_all = "camelCase")]` 保证前后端命名一致
2. **`core/`** — 纯业务逻辑层，处理计算、解析、异步 I/O，不依赖 Tauri API
3. **`commands/`** — `#[tauri::command]` 包装器，负责参数校验、调用 core、发送 Tauri 事件

---

## 许可证

MIT
