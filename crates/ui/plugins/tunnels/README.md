# plugin-tunnels: 端口转发、SOCKS5 代理与网络拓扑可视化插件

> **模块定位**：Smalux 网络隧道与跳板穿透中枢（左侧活动栏第四位）。支持纯 Rust 原生本地端口转发 (-L)、远程反向代理 (-R)、动态 SOCKS5 代理 (-D) 与多跳链式堡垒机穿透 (`TunnelsCenterView`)，提供实时拓扑卡片、网络吞吐度量条 (`TunnelMetricsBar`) 以及终端伴生快速隧道抽屉 (`TunnelToolDrawer`)。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **三大转发模式与跳板拓扑全景**：
   - **Local (-L)**：将本地监听端口绑定至远程内网数据库/微服务（如 `127.0.0.1:3306 -> 192.168.1.100:3306`）；
   - **Dynamic (-D)**：纯 Rust 原生 SOCKS5 协议，一键将本地转为透明安全代理节点；
   - **Remote (-R)**：将内网测试服务暴露给远程公网主机；
   - **Bastion 多跳链**：可视化配置跳板机链式跳转路径，节点连通性一目了然。
2. **纯 Rust 原生驱动与零进程泄漏**：
   - 底层由 `smagical-ssh::RusshTunnelDriver` 直通 Tokio 异步管道，零外部 `nc`、`connect.exe` 或 `ssh.exe` 进程拉起；
   - 原子级双向度量：实时采集各隧道的活跃连接数 (`active_connections`)、接收字节 (`bytes_in`) 与发送字节 (`bytes_out`)。
3. **终端内嵌伴生抽屉 (`companion/tunnel_tool_drawer.slint`)**：
   - 随当前终端活跃主机自动关联，一键启停当前主机的端口转发规则，无需切出工作区。

---

## 二、 核心 Slint 组件与调用方法 (Slint Components & Usage)

### 1. `TunnelsCenterView` (全屏网络与隧道中心)
```slint
import { TunnelsCenterView } from "@plugin-tunnels/tunnels_plugin.slint";

TunnelsCenterView {
    horizontal-stretch: 1;
    back-to-terminal => { WindowBridge.main-view = "terminal"; }
}
```

### 2. `TunnelToolDrawer` (终端右侧伴生隧道快速控制抽屉)
```slint
import { TunnelToolDrawer } from "@plugin-tunnels/tunnels_plugin.slint";

TunnelToolDrawer {
    width: 100%;
    height: 100%;
    has-active-terminal: TerminalBridge.has-active-session;
    toggle-tunnel-running(id) => { TunnelsBridge.toggle-tunnel(id); }
    collapse => { WindowBridge.is-right-drawer-open = false; }
}
```

### 3. `CreateHostTunnelModal` (主机专属端口转发新建模态框)
```slint
import { CreateHostTunnelModal } from "@plugin-tunnels/tunnels_plugin.slint";

CreateHostTunnelModal {
    is-open <=> TunnelsBridge.is-create-host-tunnel-modal-open;
    host-name: TunnelsBridge.active-host-name;
    save => { TunnelsBridge.save-tunnel(); }
    close => { TunnelsBridge.close-create-host-tunnel-modal(); }
}
```

---

## 三、 核心 Rust 驱动与 API 接口 (Rust Handlers & APIs)

在 `crates/smagical-ui/src/handlers/tunnel_handlers.rs` 中集中管理：

### 1. 启停网络隧道规则
```rust
pub async fn toggle_tunnel_active(
    tunnel_id: &str,
    driver: &Arc<RusshTunnelDriver>,
    storage: &Arc<dyn AppStorage>,
) -> Result<bool>
```
- **参数**：隧道规则 ID、隧道驱动引擎与持久化仓储；
- **返回值**：`true` 表示隧道已成功绑定并监听，`false` 表示已优雅释放并关闭监听端口；
- **机制**：若启用，驱动绑定本地 `TcpListener` 并拉起双向带度量的异步转发泵；若停用，安全通知所有子连接并关闭端口。

### 2. 轮询度量快照
```rust
pub async fn poll_tunnel_metrics(
    driver: &Arc<RusshTunnelDriver>,
) -> Vec<(String, TunnelMetricsSnapshot)>
```
- **返回值**：各运行中隧道的连接数、入站字节数与出站字节数，按秒同步至 `TunnelMetricsBar`。

---

## 四、 核心数据模型 (Data Models)

```rust
// 对应 Slint 中的 TunnelItemData
pub struct TunnelItemData {
    pub id: SharedString,
    pub name: SharedString,
    pub tunnel_type: SharedString, // "local" | "remote" | "dynamic"
    pub local_bind: SharedString, // "127.0.0.1:8080"
    pub remote_target: SharedString, // "10.0.0.1:80"
    pub is_active: bool,
    pub active_conns: i32,
    pub bytes_transferred_text: SharedString, // "148.5 MB"
}
```
