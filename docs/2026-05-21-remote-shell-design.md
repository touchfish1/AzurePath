# 远程 Shell 功能设计文档

## 1. 概述

在 AzurePath 中集成远程 Shell 功能，提供基于 SSH/Telnet 协议的终端会话管理、SFTP 文件传输、主机监控以及数据库/中间件管理能力。参考 [rshell](D:\opensource\rshell) 项目的成熟实现进行适配。

### 目标
- 提供类标签页的 SSH/Telnet 终端体验
- 集成 SFTP 文件浏览与下载
- 主机资源监控（CPU/内存/磁盘）
- 数据库管理（MySQL/PostgreSQL/Redis/Zookeeper/Etcd）
- 操作审计日志
- 多环境隔离

## 2. 功能分解

### 2.1 远程终端（SSH/Telnet）
- 会话管理：创建/编辑/删除 SSH/Telnet 会话
- 多标签页终端（基于 xterm.js）
- 编码支持（UTF-8/GBK 等）
- Keepalive 保活机制
- 会话克隆（同主机多标签）
- 批量关闭标签操作

### 2.2 SFTP 文件管理
- 远程目录浏览（目录/文件图标、上级导航）
- 文件下载到本地（含重名自动去重）
- 文本文件在线查看与编辑保存
- 文件上传

### 2.3 主机监控
- CPU 使用率
- 内存使用情况（已用/总量/百分比）
- 磁盘使用情况（已用/总量/百分比）
- 自动轮询与手动刷新

### 2.4 数据库与中间件管理
| 类型 | 功能 |
|------|------|
| MySQL | 连接管理、库/表/字段浏览、SQL 执行、EXPLAIN、表结构变更 |
| PostgreSQL | 连接管理、库/表/字段浏览、SQL 执行、EXPLAIN |
| Redis | 连接管理、Key 搜索、类型识别、string/hash/list/set/zset 读写、TTL 编辑、DB 切换 |
| Zookeeper | 连接管理、节点树浏览、节点数据读取与保存 |
| Etcd | 连接管理、Key-Value 浏览与编辑 |

### 2.5 审计日志
- 终端操作审计（连接/断开/命令）
- 数据库操作审计
- 审计记录筛选、导出（CSV/JSON）
- 审计统计报表

### 2.6 环境管理
- 多环境隔离（如：开发/测试/生产）
- 连接按环境分组
- 环境切换

## 3. 架构设计

### 3.1 三层 Rust 结构（遵循 AzurePath 现有模式）

```
src-tauri/src/
├── types/remote_shell/
│   ├── mod.rs
│   ├── session.rs         # 会话模型
│   ├── terminal.rs         # 终端协议抽象
│   ├── sftp.rs             # SFTP 数据模型
│   ├── host_metrics.rs     # 主机监控模型
│   └── database.rs         # 数据库连接模型
├── core/remote_shell/
│   ├── mod.rs
│   ├── ssh.rs              # SSH 连接实现
│   ├── telnet.rs           # Telnet 连接实现
│   ├── sftp.rs             # SFTP 操作实现
│   ├── metrics.rs          # 主机指标采集
│   └── session_store.rs    # 会话持久化
├── commands/remote_shell.rs # Tauri Command 适配层
```

### 3.2 核心数据模型

```rust
// 会话配置
pub struct RemoteSession {
    pub id: Uuid,
    pub environment: String,
    pub name: String,
    pub protocol: Protocol,      // Ssh | Telnet
    pub host: String,
    pub port: u16,
    pub username: String,
    pub encoding: String,        // utf-8, gbk 等
    pub keepalive_secs: u64,
    pub created_at: String,
    pub updated_at: String,
}

// 终端协议抽象 (TerminalClient trait)
pub trait TerminalClient: Send + Sync {
    async fn connect(&mut self, session: &RemoteSession) -> Result<()>;
    async fn read(&mut self) -> Result<Vec<u8>>;
    async fn write(&mut self, data: &[u8]) -> Result<()>;
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
}

// SFTP 条目
pub struct SftpEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: u64,
}

// 主机指标
pub struct HostMetrics {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_percent: f64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percent: f64,
}
```

### 3.3 状态管理

采用 AzurePath 现有的 AppState 模式：

```rust
pub struct RemoteShellState {
    // 会话列表缓存
    sessions: Arc<Mutex<Vec<RemoteSession>>>,
    // 活跃终端连接
    active_terminals: Arc<Mutex<HashMap<Uuid, Box<dyn TerminalClient>>>>,
    // 密码存储（后续可升级为加密存储）
    store: SessionStore,
}
```

### 3.4 通信方式

| 方向 | 方式 | 说明 |
|------|------|------|
| 前端调用后端 | `invoke()` | 会话 CRUD、终端操作、SFTP、监控等 |
| 后端推送前端 | `app.emit()` | 终端输出 (`remote-shell:output`)、调试日志 |

终端输出使用 Base64 编码传输，前端按会话配置的编码解码后写入 xterm.js。

## 4. 前端设计

### 4.1 路由与页面

```
/remote-shell               # 主页面 - 会话列表与终端工作台
/remote-shell/sessions      # 会话管理页面（CRUD）
/remote-shell/databases     # 数据库管理入口页
/remote-shell/databases/mysql
/remote-shell/databases/postgresql
/remote-shell/databases/redis
/remote-shell/databases/zookeeper
/remote-shell/databases/etcd
/remote-shell/audit         # 审计日志页面
```

### 4.2 页面布局

**终端工作台**：
- 左侧：会话列表（搜索过滤、在线状态指示、双击新建标签）
- 中间：标签栏 + xterm.js 终端区域
- 右侧：可选信息面板（主机监控 / SFTP 文件浏览）

### 4.3 关键前端依赖

- `xterm.js` + `xterm-addon-fit` — 终端模拟
- 现有：Vue 3 + TypeScript + Tailwind CSS

### 4.4 Pinia Store

```typescript
// stores/remoteShell.ts
interface RemoteShellState {
  sessions: RemoteSession[]
  activeTerminals: Map<string, ActiveTerminal>
  sftpEntries: SftpEntry[]
  hostMetrics: HostMetrics | null
  auditLogs: AuditRecord[]
}
```

## 5. 与现有功能的集成

| 现有功能 | 集成方式 |
|---------|---------|
| app_settings | 存储终端偏好设置（字体大小、主题等） |
| operation_history | 记录远程操作审计日志 |
| 密码管理 | 复用 `operation_history` 的 SQLite 存储，新增 `secrets` 表 |

## 6. 实施阶段

### Phase 1: 基础终端能力
- SSH/Telnet 会话管理（CRUD）
- 终端连接与 I/O
- xterm.js 集成
- 会话持久化（SQLite）

### Phase 2: SFTP 与监控
- SFTP 目录浏览与下载
- 主机指标采集与展示
- 文本文件在线编辑

### Phase 3: 数据库管理
- MySQL 连接管理与查询
- PostgreSQL 连接管理与查询
- Redis 连接管理与数据浏览
- Zookeeper 节点浏览
- Etcd 键值操作

### Phase 4: 审计与环境
- 操作审计日志
- 审计报表与导出
- 多环境管理
- 环境切换与过滤

## 7. 安全注意事项

- 密码存储：当前使用本地 JSON 文件（`secrets.json`），后续应迁移至系统凭据管理器
- 终端输出含敏感信息，操作历史存储应有访问控制
- 数据库连接字符串不应明文暴露到前端日志
- SFTP 操作需验证路径合法性，防止路径穿越

## 8. 参考实现

本设计参考 [rshell](D:\opensource\rshell) 项目的成熟架构：

- SSH 连接：使用 `ssh2` (libssh2) 库，在独立线程运行 worker，通过 channel 与异步主循环通信
- Telnet 连接：基于 `tokio::net::TcpStream` 的简单读写
- 终端协议抽象：`TerminalClient` trait 统一 SSH/Telnet 接口
- 前端终端：xterm.js + 自定义 Pane 组件
- 前端后端通信：Base64 编码终端输出 + Tauri `invoke` 轮询

## 9. 实施状态

### 已完成（v1）

| 模块 | 后端 | 前端 |
|------|------|------|
| SSH 终端连接 | `core/remote_shell/ssh.rs` — ssh2 worker 线程 | `XtermTerminal.vue` — xterm.js 封装 |
| Telnet 终端连接 | `core/remote_shell/telnet.rs` — tokio TcpStream | `XtermTerminal.vue` 复用 |
| 会话管理 CRUD | `commands/remote_shell.rs` — 6 个命令 | `SessionDialog.vue` + `SessionList.vue` |
| 终端 I/O | `send_input` / `pull_output` / `resize` | Tab 标签页 + Base64 解码轮询 |
| SFTP 文件浏览 | `list_sftp` / `read_sftp_text` / `save_sftp_text` | `SftpPanel.vue` |
| 主机监控 | `get_metrics` — SSH exec | `MetricsPanel.vue` — 进度条可视化 |
| 环境管理 | `list_environments` / `create_environment` | 环境选择器 |
| MySQL 管理 | `mysql_list_databases/tables/describe/execute_query` | `MySqlPanel.vue` — 查询编辑器 + 结果表 |
| PostgreSQL 管理 | `pg_list_databases/tables/execute_query` | `PostgreSqlPanel.vue` |
| Redis 管理 | `redis_list_keys/get_value/set_value/set_ttl` | `RedisPanel.vue` — Key 浏览器 |
| 数据库连接管理 | `list/create/delete/test_db_connection` | `DatabaseConnectionDialog.vue` |

### 待实施

- Zookeeper 节点浏览（后端类型已定义，前端未实现）
- Etcd 键值操作（后端类型已定义，前端未实现）
- 审计日志集成到 operation_history
- SFTP 文件下载到本地
- 终端编码选择（UTF-8/GBK 切换）
- SSH 私钥认证
- 会话编辑功能（前端确认对话框已就绪，待接入编辑入口）

### 关键技术栈

- **后端新增依赖**: `ssh2`, `sqlx` (MySQL + PostgreSQL), `redis`, `base64`, `async-trait`, `thiserror`
- **前端新增依赖**: `@xterm/xterm`, `@xterm/addon-fit`
- **存储**: SQLite（会话配置 + 密码），集成现有 `rusqlite`
- **构建**: 所有 40+ Tauri 命令在 `lib.rs` 中注册并使用 `generate_handler![]`
