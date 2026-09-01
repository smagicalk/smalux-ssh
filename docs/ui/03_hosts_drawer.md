# 🌲 03. 主机资产管理抽屉 (Hosts Drawer)

---

## 📌 模块概述

主机资产管理抽屉是用户的主机与分组资源中心，支持 **树状层级模式 (Tree Mode)** 与 **卡片平铺模式 (Card Mode)**，支持跨分组拖拽调序、循环引用检测、快速过滤与动态宽度扩展。

---

## 🧩 UI 组件与文件结构

- **抽屉主组件**：[`crates/smagical-ui/ui/views/left_drawers/hosts_drawer.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/left_drawers/hosts_drawer.slint)
- **通用抽屉容器**：[`crates/smagical-ui/ui/components/drawer-container.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/drawer-container.slint)
- **纯函数与树算法**：[`crates/smagical-ui/src/tree_model.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/tree_model.rs)
- **路由回调处理器**：[`crates/smagical-ui/src/handlers/host_handlers.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/handlers/host_handlers.rs)

---

## 📐 核心数据契约 (Data Contracts)

### 1. 树节点模型 (`HostTreeNode`)
```slint
export struct HostTreeNode {
    id: string,          // 唯一标识 (如 "grp-prod", "host-k8s-01")
    name: string,        // 显示名称
    is-group: bool,      // true: 分组文件夹, false: 主机资产
    parent-id: string,   // 父分组 ID (根目录为空字符串 "")
    level: int,          // 树深度缩进 (缩进 = level * 14px)
    is-expanded: bool,   // 分组是否展开
    address: string,     // 主机 IP 或域名
    port: int,           // SSH 连接端口 (默认 22)
    status: string,      // "online" | "warning" | "error" | "offline"
    ping-ms: int,        // 延迟测速 (毫秒)
    item-count: int,     // 直属子项总数 (仅 is-group 为 true 时有效)
}
```

### 2. 卡片平铺模型 (`HostItemData`)
```slint
export struct HostItemData {
    id: string,
    name: string,
    address: string,
    port: int,
    group: string,
    status: string,
    ping-ms: int,
}
```

---

## ⚡ 核心算法与交互设计

1. **多上下文自适应双击交互**：
   - 在终端工作区双击主机：调用 `on_open_host(id)`，在中央创建 SSH 终端 Tab；
   - 在文件工作区双击主机：调用 `on_open_host_files(id)`，在右栏创建 SFTP 远程文件 Tab。
2. **树形拖拽调序与防环安全机制 (`move_and_reorder_raw_node`)**：
   - 支持将主机拖入分组内部 (`inside`)、拖到节点上方 (`before`) 或下方 (`after`)；
   - **循环嵌套防护**：严格阻止将父分组拖入其自身的子分组内；
   - **自引用防护**：阻止节点拖拽入自身。
3. **内容自适应宽度 (`calculate_max_tree_width`)**：
   - 遍历可见树节点，计算 `level * 14px + text_width + badges`，动态绑定到 `tree_content_width`，使超长节点名能够自然横向滚动，不发生文字截断。
