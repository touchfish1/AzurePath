# 代码质量综合整治方案

## 概述

系统性提升 Rust 后端代码质量：消除重复模式、完善错误处理、清理 dead code。

## 1. 共享取消令牌模块

### 新增文件

`src-tauri/src/core/cancel.rs` — 取消令牌模块

### API 设计

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self { Self(Arc::new(AtomicBool::new(false))) }
    pub fn cancel(&self) { self.0.store(true, Ordering::SeqCst); }
    pub fn is_cancelled(&self) -> bool { self.0.load(Ordering::SeqCst) }
}
```

### CancelRegistry

```rust
pub struct CancelRegistry {
    tokens: RwLock<HashMap<String, CancelToken>>,
    poisoned: AtomicBool,
}
```

使用 `RwLock`（读多写少场景优于 `Mutex`），支持 poison 恢复。

### 接口

```rust
impl CancelRegistry {
    pub fn register(&self, task_id: &str) -> CancelToken;
    pub fn cancel(&self, task_id: &str) -> bool;
    pub fn is_cancelled(&self, task_id: &str) -> bool;
    pub fn unregister(&self, task_id: &str);
    pub fn take(&self, task_id: &str) -> Option<CancelToken>;
}
```

### 使用方式

```rust
// 全局单例
static CANCEL_REGISTRY: LazyLock<CancelRegistry> = LazyLock::new(CancelRegistry::new);

// 注册
let token = CANCEL_REGISTRY.register(&task_id);

// 检查取消
if CANCEL_REGISTRY.is_cancelled(&task_id) { ... }

// 取消
CANCEL_REGISTRY.cancel(&task_id);

// 清理
CANCEL_REGISTRY.unregister(&task_id);
```

### 替换范围

| 文件 | 当前类型 | 替换动作 |
|------|----------|----------|
| `commands/ping.rs` | `HashMap<String, bool>` | 迁移到 CancelRegistry |
| `commands/traceroute.rs` | `HashMap<String, AtomicBool>` | 迁移到 CancelRegistry |
| `commands/port_scan.rs` | `HashMap<String, Arc<AtomicBool>>` | 迁移到 CancelRegistry |
| `commands/network_sniffer.rs` | `HashMap<String, Arc<AtomicBool>>` | 迁移到 CancelRegistry |
| `commands/bandwidth.rs` | `HashMap<String, bool>` | 迁移到 CancelRegistry |

## 2. Error Handling 治理

### 新增辅助函数

在 `core/utils.rs` 或新文件 `core/emit.rs` 中：

```rust
use tauri::Emitter;
use serde::Serialize;
use tracing::warn;

/// Emit a Tauri event, logging a warning if it fails.
pub fn emit_or_warn<E: Serialize + Clone>(app: &AppHandle, event: &str, payload: &E) {
    if let Err(e) = app.emit(event, payload) {
        warn!("[emit] event '{}' failed: {}", event, e);
    }
}
```

### 改造范围

27 处 `let _ = app.emit(...)` 分布：

| 文件 | 数量 | 处理方式 |
|------|------|----------|
| `commands/file_transfer.rs` | 5 | 已返回 Result，可传播 |
| `commands/network_sniffer.rs` | 13 | spawn 内 → 用 emit_or_warn |
| `commands/lan.rs` | 1 | spawn 内 → 用 emit_or_warn |
| `commands/clipboard.rs` | 1 | spawn 内 → 用 emit_or_warn |
| `commands/topology.rs` | 3 | spawn 内 → 用 emit_or_warn |
| `core/file_transfer/mod.rs` | 1 | spawn 内 → 用 emit_or_warn |
| `core/monitor/mod.rs` | 3 | spawn 内 → 用 emit_or_warn |

## 3. Dead Code 清理

当前 39 处 `#[allow(dead_code)]` 分布在 18 个文件中。分类处理：

### 可删除（内部函数，无外部引用）

- `core/ping/mod.rs` 中的 `execute_ping`、`parse_ping_output` — 观察是否被 commands 引用
- `core/traceroute/mod.rs` 中的辅助函数 — 被 commands 引用则保留
- 各模块中的内部测试辅助函数

### 保留（公开 API / 测试）

- `core/connection/`、`core/bookmark.rs`、`core/clipboard/` 等导出供 commands 使用的公开函数
- 测试模块（`#[cfg(test)]`）内部的辅助函数

### 原则

- 编译器不报 `dead_code` 的代码不动（已经引用）
- 仅删除编译器标注为 dead 点且确认无外部引用的代码
- 每一步确保 `cargo check` 通过
