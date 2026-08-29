# smagical-debug

`smagical-debug` 是 **smalux-ssh** 的开发者调试工作台与诊断测试支撑 crate，负责全系统的 Tracing 日志收集与持久化、海量主机资产批量生成、多场景测试数据注入以及树形自适应布局测量。

---

## 📁 目录结构

```text
crates/smagical-debug/
├── Cargo.toml
└── src/
    ├── lib.rs              # 门面模块与公共函数导出
    ├── models.rs           # DebugLogEntry 日志条目与 DebugRawNode 调试节点
    ├── tracing_layer.rs    # Tracing 内存环形缓冲区、UI 捕获层与按天滚动文件写入
    ├── batch.rs            # 批量资产生成引擎 (BatchGenerateConfig)
    ├── presets.rs          # 场景预设引擎 (Minimal, K8s, Microservices, Large Stress)
    ├── inspector.rs        # 树形节点宽度测量算法 (calculate_node_width)
    └── logger.rs           # 便捷日志记录辅助接口
```

---

## 🧩 核心功能模块

### 1. 全局 Tracing 日志系统 (`tracing_layer.rs`)

- **非阻塞多后端分发**：同时输出至控制台、按天滚动文件（`%APPDATA%/smalux/logs`）以及内存环形缓冲区；
- **内存环形缓冲区**：保留最近 500 条结构化日志（时间戳、日志级别、来源模块、消息体），供 UI 端调试抽屉实时拉取；
- **文件生命周期自动维护**：提供 `clean_expired_logs(dir, max_days, max_files)`，启动时自动清理过期与超出数量限制的旧日志。

### 2. 场景预设注入 (`presets.rs`)

提供一键注入多样化的真实与极限业务场景：

| 预设 ID | 场景描述 | 包含资产概况 |
| :--- | :--- | :--- |
| `minimal` | 极简环境 (默认) | 2 个基础分组，4 台日常测试主机 |
| `k8s` | Kubernetes 集群 | 3 层嵌套层级（生产/测试/K8s/Etcd），包含主控节点与工作节点 |
| `microservices` | 微服务分布式架构 | 涵盖 API 网关、鉴权中心、订单/支付服务、Redis 缓存分片与 PostgreSQL 集群 |
| `large_tree` | 大规模深度嵌套压测 | 100+ 台主机资产，深度可达 5 层，测试虚拟滚动与渲染帧率 |

### 3. 批量资产生成引擎 (`batch.rs`)

```rust
use smagical_debug::{generate_batch_hosts, BatchGenerateConfig};

let config = BatchGenerateConfig {
    name_prefix: "node-worker-".to_string(),
    count: 50,
    start_index: 1,
    ip_prefix: "10.0.1.".to_string(),
    start_ip: 10,
    port: 22,
    group_name: "云原生算力池/Worker".to_string(),
    status_mode: "random".to_string(), // "all_online" / "all_offline" / "random"
};

let (tree_nodes, card_items) = generate_batch_hosts(&config);
```

### 4. 树形节点宽度动态测量 (`inspector.rs`)

基于字符数、中英文全角/半角权重、层级缩进与操作图标预留宽度，精确测量树形节点所需的物理像素宽度，驱动 Slint 横向平滑自适应滚动。

---

## 🧪 单元测试

```bash
cargo test -p smagical-debug
```
涵盖日志记录过滤、场景预设生成、批量主机生成与修改、嵌套路径解析以及过期文件清理等 6 项测试。
