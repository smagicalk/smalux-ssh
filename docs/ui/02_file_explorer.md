# 📂 02. 双盘文件浏览器 (Dual-Pane File Explorer)

---

## 📌 模块概述

双盘文件浏览器为专业运维人员提供左右双栏（本地磁盘 + 远程 SFTP 会话）的并行文件管理与高速传输体系。支持多 Tab 会话、前进/后退/向上导航、拖拽调序、文件右键菜单以及底部折叠式传输任务队列。

---

## 🧩 UI 组件与文件结构

- **主视图组件**：[`crates/smagical-ui/ui/views/file_explorer_view.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/file_explorer_view.slint)
- **主机选择弹窗**：[`crates/smagical-ui/ui/components/file-host-modal.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/file-host-modal.slint)
- **右键上下文菜单**：[`crates/smagical-ui/ui/components/context-menu.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/context-menu.slint)
- **领域数据模型**：[`crates/smagical-core/src/domain/file_item.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-core/src/domain/file_item.rs)
- **路由回调处理器**：[`crates/smagical-ui/src/handlers/file_handlers.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/handlers/file_handlers.rs)

---

## 📐 核心数据契约 (Data Contracts)

### 1. 文件/目录条目模型 (`FileItemData`)
```slint
export struct FileItemData {
    id: string,                 // 唯一绝对路径标识 (如 "F:/code/Cargo.toml")
    name: string,               // 文件/目录显示名称
    path: string,               // 完整绝对路径
    is-dir: bool,               // 是否为目录文件夹
    size-formatted: string,     // 格式化大小 (如 "2.4 MB", 目录为 "-")
    modified-formatted: string, // 格式化修改时间 (如 "2026-08-31 18:30")
    permissions: string,        // Unix 权限字符串 (如 "drwxr-xr-x")
    is-expanded: bool,          // 树状模式下是否展开
    level: int,                 // 缩进层级 (从 0 开始)
}
```

### 2. 会话 Tab 模型 (`FileTabData`)
```slint
export struct FileTabData {
    id: string,         // 会话 Tab ID (如 "ltab-1", "rtab-2")
    host-id: string,    // 关联主机 ID ("local" 代表本机)
    title: string,      // 标签标题 (如 "本地工作区", "Prod-Web-01")
    subtitle: string,   // 当前路径或 IP (如 "F:/code", "192.168.1.100")
    status: string,     // "online" | "warning" | "offline"
    is-active: bool,    // 是否处于当前激活显示状态
}
```

### 3. 传输队列任务模型 (`TransferItemData`)
```slint
export struct TransferItemData {
    id: string,              // 任务唯一 ID
    parent-id: string,       // 父文件夹任务 ID (支持树形展开嵌套)
    filename: string,        // 传输文件名
    source-path: string,     // 源路径
    target-path: string,     // 目标路径
    is-dir: bool,            // 是否为文件夹任务
    is-expanded: bool,       // 文件夹任务在队列中是否展开
    level: int,              // 缩进层级
    item-count-text: string, // 子项描述 (如 "4 项")
    direction: string,       // "upload" (本地->远程) | "download" (远程->本地)
    progress: float,         // 进度 (0.0 ~ 1.0)
    speed-text: string,      // 实时速率 (如 "1.2 MB/s")
    status: string,          // "pending" | "transferring" | "completed" | "failed" | "paused"
    size-text: string,       // 进度大小 (如 "14.2 MB / 25.0 MB")
}
```

---

## ⚡ 核心交互与设计规范

### 1. 毫秒级 Tab 拖拽重排与跨栏禁止 (Zero-Latency Reorder)
- **居中吸附**：鼠标拖拽时，半透明虚影卡片以鼠标光标为几何中心精确跟随；
- **同栏指示**：在同栏内拖拽时显示落位竖线与 `📍 移动` 标签；
- **跨栏置灰禁止**：当鼠标移出当前 Tab 栏或跨入对侧栏目时，虚影立即切换为 `0.55` 浅灰透明度并展示 `🚫 禁止` 标志，释放时瞬时安全复位；
- **即时落位**：通过轻量单项同步 `sync_local_tabs_only` 与 `sync_remote_tabs_only`，配合移除补间延迟，实现释放后 0 延迟瞬时落位。

### 2. 路径导航工具栏 (Navigation Toolbar)
- **历史后退 (◀) / 前进 (▶)**：联动 `LocalFileTabSession` / `RemoteFileTabSession` 历史栈，无历史记录时自动置灰禁用；
- **向上进入父目录 (▲)**：自动截取当前路径的 `parent()` 并刷新；
- **地址输入框**：支持直接输入路径后按回车直达；若路径不存在，触发右上方红色气泡通知（`Toast`），输入框自动平滑回退至当前有效路径；
- **刷新按钮 (🔄)**：重新扫描本地或远程目录。

### 3. 右键上下文菜单 (File & Transfer Context Menu)
- **文件/目录条目右键**：
  - 打开 / 进入目录
  - 传输至对侧栏（上传 / 下载）
  - 新建文件夹 / 新建文件
  - 复制绝对路径到剪贴板
  - 删除文件（调用本地 `std::fs::remove_*`）
  - 刷新当前目录
- **传输队列条目右键**：
  - 暂停 / 继续传输
  - 停止传输
  - 重新传输
  - 移除记录

### 4. 底部传输抽屉 (Transfer Drawer)
- 底部常驻紧凑统计栏（展示当前活动任务数与实时总速率）；
- 点击可展开全量传输任务列表，支持查看单文件进度条、传输速率与错误状态。

---

## 🌐 5. 生命周期与事件体系集成 (Event Integration)

双盘文件浏览器全面接入核心 `EventDispatcher`（详见 [07_events_and_lifecycle.md](file:///F:/code/rust/smalux-ssh/docs/ui/07_events_and_lifecycle.md)）：

1. **连接与会话事件**：
   - `FileTabOpeningEvent`：打开前责任链校验与安全审查（支持 `.abort()`）；
   - `FileTabOpenedEvent`：挂载后广播；
   - `FileTabFocusChangedEvent`：双栏焦点切换通知；
   - `FileTabClosedEvent`：释放会话资源。
2. **路径导航事件**：
   - `FileTabNavigatedEvent`：记录路径跳转与审计。
3. **安全守护与操作事件**：
   - `FileOperationBeforeEvent`：前置拦截高危删除操作（拦截保护系统根目录与核心系统文件）；
   - `FileOperationCompletedEvent`：操作审计日志。
4. **传输管道事件**：
   - `FileTransferStartedEvent`：传输开始；
   - 传输任务流式度量采集与全局 Toast 状态通知。

