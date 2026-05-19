# 剪贴板管理功能设计文档

**版本**: 1.0
**日期**: 2026-05-19
**状态**: 已批准

## 概述

在 AzurePath 中新增剪贴板管理功能，自动记录剪贴板历史（文本、图片、文件路径），支持持久化保存、搜索、收藏、一键复制和 LAN 同步。

## 架构

```
┌─────────────────────────────────────────────────────────┐
│                    Rust 后端                             │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │ClipboardMonitor│→│ClipboardStore│→│ ConnectionMgr │ │
│  │ (轮询 1.5s)   │  │  (SQLite)    │  │ (LAN 同步)    │ │
│  └──────┬───────┘  └──────┬───────┘  └───────┬───────┘ │
│         │                 │                   │         │
│         ▼                 ▼                   ▼         │
│  ┌──────────────────────────────────────────────────┐  │
│  │              Tauri Commands                      │  │
│  │  clipboard_start/stop/list/delete/toggle_favorite│  │
│  │  clipboard_copy/clear                            │  │
│  └──────────────────────┬───────────────────────────┘  │
└─────────────────────────┼──────────────────────────────┘
                          │ IPC
┌─────────────────────────┼──────────────────────────────┐
│  Vue 3 Frontend         │                              │
│  ┌──────────────────────▼───────────────────────────┐  │
│  │           /clipboard 页面                        │  │
│  │  列表 / 搜索 / 收藏 / 复制 / 清除               │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

## 存储设计

### 数据库

使用现有 `~/AzurePath/azurepath.db`，新增表：

```sql
CREATE TABLE IF NOT EXISTS clipboard_entries (
    id           TEXT PRIMARY KEY,
    content_type TEXT NOT NULL,      -- 'text' | 'image' | 'file'
    text_content TEXT,               -- 文本内容（文本类型时有效）
    image_path   TEXT,               -- 图片保存路径（图片类型时有效）
    file_paths   TEXT,               -- 文件路径列表 JSON（文件类型时有效）
    content_hash TEXT NOT NULL,      -- 内容去重哈希
    is_favorite  INTEGER DEFAULT 0, -- 收藏标志
    created_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_clipboard_created ON clipboard_entries(created_at);
CREATE INDEX IF NOT EXISTS idx_clipboard_favorite ON clipboard_entries(is_favorite);
```

### 文件存储

- 图片保存到 `~/AzurePath/clipboard/images/{uuid}.png`
- 文件路径引用，不复制文件内容

### 容量管理

- 最大 500 条记录
- 超出时自动删除最旧的非收藏条目（`is_favorite=0`）
- 收藏条目永久保留，直到用户手动取消收藏或删除

## Rust 后端模块

### 文件结构

```
src-tauri/src/
├── core/
│   ├── clipboard/
│   │   ├── mod.rs          # 模块导出
│   │   ├── monitor.rs      # ClipboardMonitor - 轮询检测
│   │   └── store.rs        # ClipboardStore - SQLite CRUD
│   └── mod.rs              # 添加 pub mod clipboard
├── types/
│   └── clipboard.rs        # ClipboardEntry 数据结构
├── commands/
│   └── clipboard.rs        # Tauri IPC 命令
└── lib.rs                  # 注册命令
```

### ClipboardMonitor

- 后台 tokio 任务，每 1.5 秒轮询一次
- 使用 `tauri-plugin-clipboard-manager` 读取剪贴板
- 按优先级依次检查：文件 > 图片 > 文本
- 计算内容哈希（文本使用 `std::hash::DefaultHasher`，图片使用文件大小+修改时间），与上次记录比较去重
- 检测到新内容时：
  1. 保存到 ClipboardStore
  2. 发送 `clipboard:new` 事件到前端
  3. 如 LAN 同步开启，通过 ConnectionManager 广播
- 提供 `start()` / `stop()` 控制

### ClipboardStore

- `new()` — 打开/创建数据库，初始化表
- `insert(entry)` — 插入条目，返回是否成功
- `list(search: Option<String>, limit: u32)` — 查询历史，收藏置顶
- `get_by_id(id) -> Option<ClipboardEntry>` — 按 ID 查询
- `delete(id)` — 删除单条
- `toggle_favorite(id) -> bool` — 切换收藏状态
- `clear()` — 清空所有记录
- `count() -> u32` — 当前条目数
- `evict_old()` — 超出 500 条时删除最旧非收藏条目

### LAN 同步

- 在 `Frame` 枚举中新增 `ClipboardSync { entries: Vec<ClipboardEntry> }`
- 剪贴板变化时，通过 `conn_mgr.broadcast()` 广播给所有在线设备
- 对端收到后在 `handle_frame` 中保存到本地 ClipboardStore
- 发送 `clipboard:synced` 事件通知前端

### Tauri 命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `clipboard_start` | — | `Result<(), String>` | 启动监听 |
| `clipboard_stop` | — | `Result<(), String>` | 停止监听 |
| `clipboard_list` | `search?: string, limit?: number` | `Vec<ClipboardEntry>` | 查询历史 |
| `clipboard_delete` | `id: string` | `Result<(), String>` | 删除条目 |
| `clipboard_toggle_favorite` | `id: string` | `Result<bool, String>` | 切换收藏 |
| `clipboard_copy` | `id: string` | `Result<(), String>` | 复制到系统剪贴板：文本类型复制文本，图片类型从磁盘读取后复制图片，文件类型复制文件路径列表 |
| `clipboard_clear` | — | `Result<(), String>` | 清空所有 |

### 事件

| 事件 | Payload | 说明 |
|------|---------|------|
| `clipboard:new` | `ClipboardEntry` | 新剪贴板条目 |
| `clipboard:synced` | `ClipboardEntry` | 来自 LAN 同步的条目 |

## 数据类型

```rust
#[derive(Serialize, Deserialize, Clone)]
struct ClipboardEntry {
    id: String,
    content_type: String,    // "text" | "image" | "file"
    text_content: Option<String>,
    image_path: Option<String>,
    file_paths: Option<Vec<String>>,
    content_hash: String,
    is_favorite: bool,
    created_at: String,
}
```

## 前端页面

### 路由

- 路径: `/clipboard`
- 侧边栏图标: `Clipboard` (lucide-vue-next)

### 布局

```
┌──────────────────────────────────────────────┐
│ 剪贴板管理              [🔍 搜索] [🗑️ 清除] │
├──────────────────────────────────────────────┤
│                                              │
│ ★ 📄 复制的文本内容摘要...           📋 复制  │
│   2026-05-19 14:30                            │
│  ─────────────────────────────────────────    │
│ ☆ 🖼️ [缩略图]                        📋 复制  │
│   2026-05-19 14:28                            │
│  ─────────────────────────────────────────    │
│ ☆ 📁 report.pdf + 2 个文件           📋 复制   │
│   2026-05-19 14:25                            │
│                                              │
│  ── 共 3 条记录 | 正在监听 ──               │
└──────────────────────────────────────────────┘
```

### 功能

- **列表**: 按时间倒序，收藏条目置顶
- **内容预览**: 文本截断前 100 字符，图片显示缩略图，文件显示文件名列表
- **搜索**: 按 `text_content` 模糊匹配，实时过滤
- **收藏**: 点击 ☆/★ 切换，调用 `clipboard_toggle_favorite`
- **一键复制**: 点击"复制"按钮调用 `clipboard_copy`，按钮变"已复制!" 2 秒
- **清除**: 点击清除按钮弹出确认对话框，调用 `clipboard_clear`
- **监听状态**: 页脚显示"正在监听" / "已暂停" / "剪贴板未启动"
- **实时更新**: 监听 `clipboard:new` 事件，新条目自动插入列表顶部

## 依赖

- `tauri-plugin-clipboard-manager` — Tauri 2.0 剪贴板插件
- `sha2` — 内容哈希去重（可选，也可用简单哈希）
