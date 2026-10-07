# plugin-hosts: 主机资产管理与伴生运维工具插件

> **模块定位**：Smalux 的首要核心插件（左侧活动栏第一位）。负责管理全量 SSH / 本地主机资产无限层级树形结构，提供高性能资产过滤检索、主机增删改查表单、以及三大专属伴生运维抽屉：**AI 运维对话助手 (`ai/`)**、**Linux 实时性能探针 (`monitor/`)** 与 **Tmux 会话纳管器 (`tmux/`)**。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **多层级无限树形资产抽屉 (`drawers/hosts_drawer.slint`)**：
   - 资产树支持无限级嵌套文件夹（分组）折叠/展开、超宽节点横向平滑滚动；
   - 零分配毫秒级模糊搜索（基于 `matches_any_ignore_case`），输入即过滤；
   - 节点状态包含连通性延迟 Ping (ms)、在线/离线/告警状态指示灯。
2. **主机资产与分组录入模态框 (`modals/`)**：
   - **`CreateHostModal`**：支持主机名、IP/域名、端口（默认 22）、关联凭据（密码/私钥）、跳板机链式代理与高级超时参数配置；
   - **`CreateGroupModal`**：支持树状父分组选择与分组重命名。
3. **独立伴生工具集 (`companion/`)**：
   - **AI 智能助手 (`companion/ai/`)**：支持流式生成、命令风险高亮研判（安全/注意/高危）、一键投递执行与思考过程折叠；
   - **实时性能探针 (`companion/monitor/`)**：轻量级单行复合指令采样，解析 CPU 核心使用率、内存物理占比、网络双向吞吐折线图（Sparkline）、挂载磁盘容量表与 TOP 进程表；
   - **Tmux 伴生器 (`companion/tmux/`)**：可视化列出远程所有活跃 Tmux 会话，支持一键 Attach/Detach、新建窗口与水平/垂直分屏。

---

## 二、 核心 Slint 组件与调用方法 (Slint Components & Usage)

### 1. `HostsDrawer` (左侧主机树抽屉)
```slint
import { HostsDrawer } from "@plugin-hosts/hosts_plugin.slint";

HostsDrawer {
    width: 100%;
    height: 100%;
    collapse => { WindowBridge.is-left-drawer-open = false; }
    host-double-clicked(id) => { HostsBridge.open-host(id); }
    open-local-shell(shell-type) => { HostsBridge.open-host("local-" + shell-type); }
}
```

### 2. `MonitorDrawer` (右侧性能监控抽屉)
```slint
import { MonitorDrawer } from "@plugin-hosts/hosts_plugin.slint";

MonitorDrawer {
    width: 100%;
    height: 100%;
    metrics: MonitorBridge.metrics;
    has-active-host: TerminalBridge.has-active-session;
    collapse => { WindowBridge.is-right-drawer-open = false; }
}
```

### 3. `AiToolDrawer` (右侧 AI 对话抽屉)
```slint
import { AiToolDrawer } from "@plugin-hosts/hosts_plugin.slint";

AiToolDrawer {
    width: 100%;
    height: 100%;
    has-active-host: TerminalBridge.has-active-session || AiBridge.has-active-host;
    collapse => { WindowBridge.is-right-drawer-open = false; }
}
```

---

## 三、 核心 Rust 驱动与 API 接口 (Rust Handlers & APIs)

在 `crates/smagical-ui/src/handlers/host_handlers.rs` 中集中管理：

### 1. 异步拉起 SSH 连接管线
```rust
pub fn spawn_ssh_connection_pipeline(
    session_id: String,
    host: HostRecord,
    credential: Option<CredentialRecord>,
    size: TerminalSize,
    tx_output: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    tx_disconnect: tokio::sync::mpsc::UnboundedSender<String>,
    storage: Arc<dyn AppStorage>,
    ssh_driver: Arc<RusshSessionDriver>,
)
```
- **参数**：
  - `session_id`: 目标标签页会话全局唯一 ID；
  - `host`: 主机实体（含保活、超时与代理设置）；
  - `credential`: 解密后的凭据（若有）；
  - `size`: 初始终端行列尺寸（如 80x24）；
  - `tx_output`: PTY 输出数据流管道；
  - `tx_disconnect`: 会话异常断开通知通道。
- **说明**：在 Tokio 后台任务中异步完成 TCP 三次握手、跳板机中继、SSH 密钥协商与 PTY Channel 开启，支持 15s 心跳保活与智能重连。

### 2. 主机搜索与过滤 (ASCII 零分配快径)
```rust
// tree_model.rs
pub fn filter_tree_nodes(
    nodes: &[HostTreeNode],
    query: &str,
) -> Vec<HostTreeNode>
```
- **参数**：全量主机树节点切片与搜索词；
- **优化**：全线走 `matches_any_ignore_case`，命中父节点时自动保留子树，命中叶子节点时自动展开其全链路上级。

---

## 四、 核心数据模型 (Data Models)

```rust
// 对应 Slint 中的 HostTreeNode
pub struct HostTreeNode {
    pub id: SharedString,
    pub name: SharedString,
    pub address: SharedString,
    pub port: i32,
    pub is_group: bool,
    pub parent_id: SharedString,
    pub level: i32,
    pub is_expanded: bool,
    pub item_count: i32,
    pub ping_ms: i32,
    pub status: SharedString, // "online" | "warning" | "error" | "offline"
}
```
