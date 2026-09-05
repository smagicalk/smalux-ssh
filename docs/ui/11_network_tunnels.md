# 🌐 网络隧道、跳板机与出网代理 (Network Tunnels & Proxy)

本文档详细阐述 `smalux-ssh` 桌面客户端**网络隧道中心 (TunnelsCenterView)**、左右辅助抽屉、三大核心网络资产模型、拓扑可视化及领域桥接 (`TunnelBridge`) 数据契约。

---

## 一、模块定位与界面架构

网络隧道中心为多云与异构网络运维提供一站式链路编排支持，界面采用高沉浸度 **Master-Detail 弹性双栏架构**：

- **左侧资产列表 (320px)**：
  - 顶部快速操作栏：模糊搜索输入框、资产分类过滤胶囊 (`AppSegmentedControl`: 全部 / 端口转发 / 跳板堡垒 / 出网代理)；
  - 资产卡片流：展示规则名称、类型徽章、监听地址:端口、目标端点、连接状态微点 (`AppStatusDot`) 与瞬时吞吐速率；
  - 底部快速新建按钮：唤出资产类型选择弹窗 (`CreateTunnelTypeModal`)。
- **右侧拓扑与配置详情视口**：
  - **顶部拓扑概览 (`TunnelTopologyCard`)**：实时动态绘制 `客户端 ➔ (跳板机) ➔ 目标服务器` 连线状态、心跳波形图与当前连接数；
  - **中部多态表单区**：根据当前选中的资产类型动态切换为专用配置表单；
  - **底部操作控制条**：一键启停开关、链路测试、复制规则、删除规则。

```text
+---------------------------------------------------------------------------------------------------------+
| [🌐 网络隧道与代理中心]                                         [+ 新建隧道]   [✕ 返回工作台 / ESC]       |
+------------------------------+--------------------------------------------------------------------------+
| 🔍 搜索规则名称/端口...      | ╭─ 链路拓扑与实时状态 ──────────────────────────────────────────────────╮ |
| [ 全部 | 转发 | 跳板 | 代理 ] | │ 本地 [127.0.0.1:8080] ──────► SSH [跳板: 10.0.0.1] ──────► 目标 [192.168.1.5:80]  │ │
+------------------------------+ │ 状态: [🟢 运行中]   活跃连接: 4   上行: 12.4 KB/s   下行: 84.2 KB/s    │ │
| ⚡ MySQL-Dev-Tunnel          | ╰────────────────────────────────────────────────────────────────────────╯ │
|    本地 3306 -> 10.0.1.2     | ╭─ 端口转发配置 ────────────────────────────────────────────────────────╮ │
|    🟢 运行中  · 12.4 KB/s    | │ 转发模式:   (•) 本地转发 (L)   ( ) 远程反向 (R)   ( ) 动态网关 (D)   │ │
|                              | │ 本地监听:   127.0.0.1 : [ 3306 ]                                     │ │
| 🛡️ Prod-Bastion-Jump         | │ 宿主服务器: [ Prod-K8s-Master (10.0.0.1) ▾ ] [选择主机]              │ │
|    跳板机 · 2 级跳转         | │ 目标地址:   10.0.1.2  : [ 3306 ]                                     │ │
|                              | ╰────────────────────────────────────────────────────────────────────────╯ │
| 🚀 Office-SOCKS5-Outbound    | [ ⏹ 停止隧道 ]           [ 🔍 连通性测试 ]            [ 💾 保存配置 ]    |
+------------------------------+--------------------------------------------------------------------------+
```

---

## 二、三大核心网络资产模型

### 1. ⚡ 端口转发资产 (Port Forwarding)
- **本地端口转发 (Local Forward, `-L`)**：将本地计算机的指定端口映射到远程 SSH 服务器可访问的内网目标主机与端口（如内网数据库、私有 Web 后台）；
- **远程反向转发 (Remote Reverse, `-R`)**：将远程 SSH 宿主机的指定端口反向映射回本地或本地所在局域网（实现无公网 IP 内网穿透）；
- **动态 SOCKS5 代理网关 (Dynamic SOCKS5, `-D`)**：在本地开启标准 SOCKS5 代理服务器，所有流量均经由 SSH 宿主机加密外出；
- **反向动态网关 (Reverse Dynamic)**：支持高级穿透路由编排。

### 2. 🛡️ 跳板机堡垒链路 (Bastion Jump Hop)
- 支持多跳（Multi-Hop）链式代理配置；
- 支持为跳板机指定独立的认证凭据（私钥、证书、Passphrase）；
- 具备自动化链路探测与断线指数退避重连。

### 3. 🚀 静态出网代理节点 (Outbound Proxy Nodes)
- 集中管理运维所需的 HTTP/HTTPS/SOCKS5 静态代理节点；
- 具备自动定时 Ping / TCP 握手健康检查，显示实时延迟 (ms) 与丢包率；
- 可直接被全局设置中心引用作为默认出网路由。

---

## 三、专属模态交互与组件

1. **新建资产类型选择弹窗 (`CreateTunnelTypeModal`)**：
   - 包含端口转发、跳板机、出网代理三大类型图文卡片；
   - 支持键盘快捷数字键快速选中创建；
2. **SSH 宿主选择器 (`HostPickerList`)**：
   - 树形多级选择与模糊搜索现有主机资产；
   - 直观呈现主机在线状态与延迟。

---

## 四、Slint 领域桥接契约 (`TunnelBridge`)

```slint
// ui/features/tunnels/tunnel_bridge.slint
export struct TunnelItemData {
    id: string,
    name: string,
    tunnel-type: string, // "forward_local", "forward_remote", "forward_dynamic", "bastion", "proxy"
    local-addr: string,
    local-port: int,
    remote-host: string,
    remote-port: int,
    host-id: string,
    host-name: string,
    is-active: bool,
    uptime: string,
    upload-speed: string,
    download-speed: string,
    notes: string,
}

export global TunnelBridge {
    in-out property <[TunnelItemData]> tunnel-list: [];
    in-out property <string> active-tunnel-id: "";
    in-out property <string> filter-category: "all";
    in-out property <string> search-query: "";
    
    // 操作回调
    callback toggle-tunnel(string /* id */, bool /* target_active */);
    callback save-tunnel(TunnelItemData);
    callback delete-tunnel(string /* id */);
    callback test-connectivity(string /* id */);
    callback duplicate-tunnel(string /* id */);
}
```
