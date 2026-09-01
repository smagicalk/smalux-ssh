# 🕒 04. 历史会话中心 (History Center)

---

## 📌 模块概述

历史会话模块负责记录所有建立过的终端连接快照，支持 **左侧辅助抽屉 (Drawer View)** 与 **全屏独立历史中心大页面 (Fullscreen Center View)** 两种形态，提供按时间、按主机、按模式聚合查看与快捷分屏重连能力。

---

## 🧩 UI 组件与文件结构

- **全屏中心视图**：[`crates/smagical-ui/ui/views/history_center_view.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/history_center_view.slint)
- **左侧抽屉组件**：[`crates/smagical-ui/ui/views/left_drawers/history_drawer.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/left_drawers/history_drawer.slint)
- **路由回调处理器**：[`crates/smagical-ui/src/handlers/history_handlers.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/handlers/history_handlers.rs)
- **存储与持久化**：[`crates/smagical-core/src/storage/mock_storage.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-core/src/storage/mock_storage.rs)

---

## 📐 核心数据契约 (Data Contracts)

### 1. 历史会话分组模型 (`HistoryGroupData`)
```slint
export struct HistoryGroupData {
    group-id: string,       // 分组 ID (如 "today", "yesterday", "earlier")
    group-title: string,    // 分组显示标题 (如 "📅 今天 (Today)", "🌐 生产集群")
    item-count: int,        // 该分组包含的记录数
    is-collapsed: bool,     // 是否折叠
    items: [HistoryItemData], // 会话条目列表
}
```

### 2. 历史会话条目模型 (`HistoryItemData`)
```slint
export struct HistoryItemData {
    id: string,               // 历史记录唯一 ID
    host-id: string,          // 关联主机 ID
    title: string,            // 会话标题 (如 "Web-Server-01")
    subtitle: string,         // 连接地址 (如 "192.168.1.100:22")
    session-type: string,     // "ssh" | "local" | "sftp"
    timestamp-formatted: string, // 格式化时间 (如 "14:25:30")
    duration-formatted: string,  // 持续时长 (如 "25m 12s")
    is-pinned: bool,          // 是否置顶加星 ⭐
    icon-name: string,        // "server" | "terminal" | "folder"
}
```

---

## ⚡ 核心功能与操作

1. **多维度聚合视图模式 (View Modes)**：
   - **`time` (按时间分组)**：今天、昨天、更早；
   - **`host` (按主机资产分组)**：根据相同的主机聚合历史记录；
   - **`flat` (平铺时间线)**：纯时间倒序排列。
2. **快捷会话重连**：
   - 单击重连：直接激活并打开全新会话 Tab；
   - 分屏重连 (`reconnect_split`)：在当前焦点窗格旁自动创建右侧分屏并建立连接。
3. **置顶管理与安全清空**：
   - 支持单条会话置顶 / 取消置顶；
   - 支持单条删除与全量清空历史（保留置顶记录）。
