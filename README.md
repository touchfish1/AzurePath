# AzurePath

AzurePath 是一款跨平台桌面端内网运维工具箱，基于 Tauri 2.0 构建。

## 功能

- **📡 Ping** — ICMP ping 检测，支持自定义次数、间隔、超时，跨平台中英文解析
- **🗺️ Traceroute** — 路由追踪，逐跳探测途经节点，支持 Windows/Unix 输出解析
- **🔍 端口扫描** — TCP 并发端口扫描，支持自定义范围和并发数
- **🌐 DNS 查询** — 支持 A / AAAA / CNAME / MX / NS / SOA / TXT / ALL 记录类型，支持自定义 DNS 服务器
- **💬 LAN 聊天** — 局域网设备发现 + 点对点即时消息
- **📁 文件传输** — 局域网内点对点文件直传，独立文件传输管理页面
- **📋 剪贴板管理** — 剪贴板历史记录、持久化存储、搜索、收藏、跨设备同步、图片缩略图预览
- **📡 网络嗅探** — 局域网设备发现、端口扫描、服务 Banner 抓取、指纹识别、OS 探测、结果导出
- **📊 活动概览** — 首页聚合展示剪贴板、设备发现、活跃节点等近期动态

## 技术栈

| 层 | 技术 |
|------|----------|
| 桌面框架 | Tauri 2.0 |
| 后端语言 | Rust (tokio 异步) |
| 前端框架 | Vue 3 + TypeScript |
| UI 组件 | shadcn-vue + Tailwind CSS |
| 状态管理 | Pinia |
| 路由 | Vue Router |
| 数据库 | SQLite (rusqlite) |

## 开发

### 环境要求

- [Rust](https://www.rust-lang.org/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- 根据 Tauri 2.0 文档配置好的系统依赖

### 启动

```bash
# 安装前端依赖
npm install

# 启动开发服务器
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

## 项目结构

```
azurepath/
├── src/                          # 前端源码
│   ├── components/               # 通用 UI 组件
│   ├── pages/                    # 页面
│   ├── lib/                      # 工具函数和 Tauri 绑定
│   ├── router/                   # 路由配置
│   ├── stores/                   # Pinia 状态管理
│   └── App.vue                   # 根组件
├── src-tauri/                    # 后端源码 (Rust)
│   ├── src/
│   │   ├── commands/             # Tauri 命令层
│   │   ├── core/                 # 核心网络引擎
│   │   └── types/                # 数据模型
│   └── Cargo.toml
└── docs/                         # 设计文档和计划
```

## 许可证

MIT
