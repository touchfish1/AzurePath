# MTR (My TraceRoute) 网络诊断工具

## 概述

MTR (My TraceRoute) 是一款结合 Traceroute 和 Ping 的网络诊断工具。它先探测到目标的路由路径，然后持续对每一跳发送探测包，实时统计每跳的延迟、丢包率等指标，帮助定位网络瓶颈和故障点。

## 架构

### 模块结构

```
src-tauri/src/
├── types/mtr.rs          — 数据结构定义
├── core/mtr/mod.rs       — 核心逻辑（路径发现 + 轮询探测 + 统计聚合）
├── commands/mtr.rs       — Tauri 命令层

src/
├── pages/mtr/index.vue   — MTR 页面（实时 + 报告双视图）
├── stores/mtr.ts         — Pinia store
```

### 数据流

```
mtr_start(options)
  ↓
Phase 1: 路径发现
  ├── 调用系统 traceroute (tracert / traceroute)
  └── 解析输出 → Vec<HopInfo> { hop, addr, hostname }
  ↓
Phase 2: 轮询探测 (循环直到停止)
  ├── 每轮对每跳并发执行 ping -n 1
  ├── 聚合运行统计 (min/avg/max/jitter/loss%)
  ├── emit("mtr:progress", MtrProgress) — 前端实时更新
  └── 间隔 interval_ms 后进入下一轮
  ↓
mtr_stop(task_id)
  ├── 设置取消标志
  └── emit("mtr:complete", MtrComplete) — 前端切换报告视图
```

## 数据结构

### MtrOptions (输入参数)

```rust
struct MtrOptions {
    target: String,       // 目标 IP 或域名
    max_hops: u32,        // 最大跳数 (默认 30)
    interval_ms: u64,     // 轮次间隔 (默认 1000)
    timeout_ms: u64,      // 每跳探测超时 (默认 3000)
}
```

### MtrHopStats (每跳统计)

```rust
struct MtrHopStats {
    hop: u32,                  // 跳数
    addr: Option<String>,      // IP 地址
    hostname: Option<String>,  // 主机名
    sent: u32,                 // 已发送探测数
    received: u32,             // 已接收回复数
    loss_percent: f64,         // 丢包率 (%)
    min_ms: f64,               // 最小延迟
    avg_ms: f64,               // 平均延迟
    max_ms: f64,               // 最大延迟
    jitter_ms: f64,            // 抖动 (均偏差)
    last_ms: Option<f64>,      // 上一轮延迟
}
```

### MtrProgress (轮次进度事件)

```rust
struct MtrProgress {
    target: String,
    total_hops: u32,
    round: u32,
    hops: Vec<MtrHopStats>,
}
```

### MtrComplete (完成事件)

```rust
struct MtrComplete {
    target: String,
    total_rounds: u32,
    hops: Vec<MtrHopStats>,
}
```

## 复用基础设施

- **Ping 模块** (`core/ping/`): 复用 `execute_ping()` 和 `parse_ping_line()` 做每跳探测，复用 `compute_stats()` 做统计聚合
- **Traceroute 模块** (`core/traceroute/`): 复用 `execute_traceroute()` 和 `parse_traceroute_output()` 做路径发现
- **取消模式**: 复用 `CANCEL_TOKENS` + `AtomicBool` 模式

## 核心算法

### Jitter 计算

使用平均偏差 (Mean Deviation) 作为 jitter 指标：

```
jitter = Σ|latency_i - avg| / n
```

### Probe Method

出于可移植性考虑，使用系统 `ping` 命令而非原始 ICMP 套接字。每轮对每跳执行 `ping -n 1`（Windows）或 `ping -c 1`（Unix），这不需要管理员权限。

## 前端设计

### MTR 页面

- **控制栏**: 目标输入框 + 参数设置（可折叠）+ 开始/停止按钮
- **标签切换**: [实时数据] [分析报告]
- **实时视图**: 表格，每跳一行，每秒刷新
  - 列: 序号, IP, 主机名, Loss%, Snt, Last, Avg, Best, Wrst, Stdev
  - 丢包率 > 0 的行高亮标记
- **报告视图**: 停止后切换到此标签
  - 展示完整的最终统计
  - 支持导出（CSV/JSON）

### 路由

- 路径: `/mtr`
- 侧栏: 添加 "MTR 路由追踪" 导航项

### Pinia Store

```typescript
interface MtrState {
  isRunning: boolean;
  target: string;
  options: MtrOptions;
  hops: MtrHopStats[];
  totalRounds: number;
}
```

## 错误处理

- 无效目标地址 → 返回错误提示
- 目标不可达 → 路径发现阶段报错，不再进入轮询
- 某跳超时 → 该跳标记为 timeout，不影响其他跳，轮询继续
- 取消 → 立即停止，返回已采集的部分数据

## 测试策略

- 单元测试: parse_traceroute_output、compute_stats、jitter 计算
- 集成测试: 模拟 ping/traceroute 输出，验证完整流程
