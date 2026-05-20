# 子网计算器增强设计

## 概述

对工具箱中已有的子网计算器进行增强：支持 IPv6、子网划分、IP 地址分类，并将 CIDR 解析逻辑提取到 Rust 后端共享。

## 架构变化

### 当前状态

- 子网计算逻辑在 `src/pages/toolbox/Page.vue` 中纯前端 JavaScript 实现
- `parse_cidr()` 函数在 `src-tauri/src/commands/network_sniffer.rs` 中重复实现

### 目标状态

```
src-tauri/src/
├── core/subnet/mod.rs     — 新增: CIDR 解析 (IPv4 + IPv6)、子网计算、子网划分、IP 分类
├── types/subnet.rs        — 新增: 数据结构
├── commands/...           — 新增 Tauri 命令 (calculate_subnet, split_subnet)
├── commands/network_sniffer.rs — 修改: 引用 core/subnet 的 parse_cidr

src/
└── pages/toolbox/Page.vue — 增强: 子网计算 Tab 添加新功能
```

## 数据结构

### SubnetInput

```rust
struct SubnetInput {
    address: String,   // "192.168.1.0" 或 "2001:db8::"
    cidr: u8,          // 24 或 64
}
```

### SubnetResult

```rust
struct SubnetResult {
    network_address: String,
    broadcast_address: String,   // IPv6 为 ""
    subnet_mask: String,
    wildcard_mask: String,
    usable_hosts: u64,           // IPv6 显示为 "大量" 或具体数字
    ip_range: String,
    cidr: u8,
    ip_version: String,          // "IPv4" | "IPv6"
    classification: IpClassification,
}
```

### IpClassification

```rust
struct IpClassification {
    is_private: bool,
    is_loopback: bool,
    is_link_local: bool,
    is_multicast: bool,
    is_public: bool,
    description: String,
}
```

### SubnetSplitResult

```rust
struct SubnetSplitResult {
    subnets: Vec<SubnetResult>,
    total_usable: u64,
}
```

## 核心逻辑 (core/subnet/)

### IPv4 CIDR 解析

从 `network_sniffer.rs` 提取现有的 `parse_cidr()`，保持行为不变。

### IPv6 CIDR 解析

新增 `parse_cidr_v6()`:

- 解析 IPv6 地址 + 前缀长度（如 `2001:db8::/32`）
- 计算网络地址、接口标识符范围
- 注意 IPv6 没有广播地址概念
- 对于 /64 及更大的子网，可用主机数为 2^(128-prefix)，显示为科学计数法或 "大量"

### IP 分类逻辑

根据 RFC 规则分类 IPv4 地址：

| 范围 | 分类 |
|------|------|
| 10.0.0.0/8 | 私有 (RFC 1918) |
| 172.16.0.0/12 | 私有 (RFC 1918) |
| 192.168.0.0/16 | 私有 (RFC 1918) |
| 127.0.0.0/8 | 环回 |
| 169.254.0.0/16 | 链路本地 |
| 224.0.0.0/4 | 多播 |
| 240.0.0.0/4 | 保留 |
| 其余 | 公网 |

IPv6 分类：

| 范围 | 分类 |
|------|------|
| ::1/128 | 环回 |
| fe80::/10 | 链路本地 |
| fc00::/7 | 唯一本地地址 (ULA) |
| ff00::/8 | 多播 |
| 2000::/3 | 全局单播 (公网) |

### 子网划分

输入 `192.168.1.0/24` 和目标前缀 `/26`，输出 4 个子网：

1. 192.168.1.0/26 (192.168.1.1 - 192.168.1.62)
2. 192.168.1.64/26 (192.168.1.65 - 192.168.1.126)
3. 192.168.1.128/26 (192.168.1.129 - 192.168.1.190)
4. 192.168.1.192/26 (192.168.1.193 - 192.168.1.254)

校验：目标前缀必须大于输入前缀（子网划分），且在合法范围内。

## Tauri 命令

```rust
#[tauri::command]
fn calculate_subnet(address: String, cidr: u8) -> Result<SubnetResult, String>

#[tauri::command]
fn split_subnet(network: String, target_prefix: u8) -> Result<SubnetSplitResult, String>
```

在 `lib.rs` 的 `generate_handler![]` 中注册。

## 前端增强

### 子网计算 Tab

现有面板基础上增加：

1. **IPv4/IPv6 切换开关** — 切换地址输入提示和计算逻辑
2. **IP 分类标签** — 在结果显示区增加一个彩色标签（如 "私有地址 RFC 1918"）
3. **子网划分面板** — 新增一个可折叠区域
   - 输入目标前缀
   - 点击"划分子网"
   - 以表格形式显示所有子网结果

## 错误处理

- IPv4: 无效格式、CIDR > 32、范围过大（限制 /16）— 复用现有逻辑
- IPv6: 无效格式、CIDR > 128 — 返回明确错误
- 子网划分: target_prefix <= input_prefix 时报错

## 测试

- 单元测试: parse_ipv4_cidr、parse_ipv6_cidr、ipv4_classify、ipv6_classify
- 子网划分验证: 确保子网不重叠、覆盖完整范围
- 从 network_sniffer 移植现有 parse_cidr 测试到公共模块
