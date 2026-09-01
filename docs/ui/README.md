# 🎨 Smalux-SSH UI 架构与设计规范总览 (UI Architecture & Design System)

本文档作为 `smalux-ssh` 桌面端 UI 系统的总体设计规范与模块索引。

---

## 📑 模块文档索引 (Page-by-Page Documentation)

| 序号 | 模块名称 | 文档链接 | 核心职责 |
| :---: | :--- | :--- | :--- |
| **01** | **终端会话工作区** | [01_terminal_workspace.md](file:///F:/code/rust/smalux-ssh/docs/ui/01_terminal_workspace.md) | 多窗格无限分屏、Tab 标签栏、PTY 进程托管与 120Hz 高性能光栅化渲染引擎 |
| **02** | **双盘文件管理器** | [02_file_explorer.md](file:///F:/code/rust/smalux-ssh/docs/ui/02_file_explorer.md) | 左右独立双栏 Tab、毫秒级 Tab 拖拽重排、路径导航直达、文件列表与传输队列抽屉 |
| **03** | **主机资产管理抽屉** | [03_hosts_drawer.md](file:///F:/code/rust/smalux-ssh/docs/ui/03_hosts_drawer.md) | 树形层级 / 卡片平铺双模式、拖拽调序与防环检测、动态计算内容宽度 |
| **04** | **历史会话中心** | [04_history_center.md](file:///F:/code/rust/smalux-ssh/docs/ui/04_history_center.md) | 侧边栏历史抽屉与全屏独立历史中心、按时间/主机/模式聚合、会话置顶与重连 |
| **05** | **全局通用组件库** | [05_global_components.md](file:///F:/code/rust/smalux-ssh/docs/ui/05_global_components.md) | 全局气泡通知 (Toast)、右键上下文菜单 (ContextMenu)、主机选择列表 (HostPickerList)、模态弹窗体系 |
| **06** | **开发者调试控制台** | [06_debug_console.md](file:///F:/code/rust/smalux-ssh/docs/ui/06_debug_console.md) | 资产批量造数、健康状态快速更新、预设写入与 Tracing 实时日志流同步 |
| **07** | **事件分发与生命周期** | [07_events_and_lifecycle.md](file:///F:/code/rust/smalux-ssh/docs/ui/07_events_and_lifecycle.md) | 泛型事件分发总线 (`EventDispatcher`)、多作用域管理器 (`EventManager`) 与生命周期守卫 |
| **08** | **凭据保管库与安全认证** | [08_credentials_vault.md](file:///F:/code/rust/smalux-ssh/docs/ui/08_credentials_vault.md) | SSH 密钥/口令安全保管、多算法密钥生成器、随机密码生成器与机密提取审计 |

---

## 📐 设计系统与视觉规范 (Design System)

### 1. 纯净无彩度暗色风格 (Achromatic Dark Theme)
Smalux-SSH 默认采用纯净无彩度暗色系，降低长时间运维工作中的视觉疲劳：

- **主背景 (`background`)**：`#18181b` (Zinc-900)
- **面板背景 (`panel-background`)**：`#121214` (深层侧边与抽屉基底)
- **表面背景 (`surface-background`)**：`#202024` (卡片、输入框与条目底色)
- **悬浮高亮 (`hover-background`)**：`#27272a` (Zinc-800)
- **边框与分割线 (`border`)**：`#2e2e33` (高对比度精细分割线)
- **主要文字 (`foreground`)**：`#f4f4f5` (Zinc-100)
- **次要文字 (`secondary-foreground`)**：`#a1a1aa` (Zinc-400)
- **禁用文字 (`disabled-foreground`)**：`#71717a` (Zinc-500)

### 2. 状态语义配色 (Status Colors)
- 🟢 **在线 / 成功 (`Success`)**：`#10b981` (Emerald-500)
- 🔵 **信息 / 强调 (`Info / Accent`)**：`#3b82f6` (Blue-500)
- 🟡 **警告 (`Warning`)**：`#f59e0b` (Amber-500)
- 🔴 **危险 / 错误 (`Error`)**：`#ef4444` (Red-500)

---

## 🥞 全局组件层级规范 (Z-Index Hierarchy)

为避免弹窗、抽屉与气泡通知之间发生遮挡穿透与焦点紊乱，统一约束如下层级：

```text
z: 1000  ───► 全局气泡通知浮层 (ToastContainer，顶层非阻塞提示)
z: 900   ───► 模态弹窗体系 (NewSessionModal, FileHostModal, DebugModal 等)
z: 800   ───► 右键上下文菜单浮层 (ContextMenuContainer)
z: 700   ───► 拖拽跟随虚影层 (TabDragGhost, FileDragGhost, HostDragGhost)
z: 100   ───► 底部辅助抽屉 (TransferQueueDrawer, TerminalDebugDrawer)
z: 10    ───► 二级可伸缩侧边抽屉 (HostsDrawer, HistoryDrawer 等)
z: 1     ───► 中央主工作区 (TerminalViewport, FileExplorerView)
```

---

## 🏛️ 前后端交互与数据流规范

```text
┌─────────────────────────────────────────────────────────────┐
│                       Slint UI 表现层                       │
│  (Views, Components, Drag TouchAreas, Callbacks, Models)    │
└──────────────────────────────┬──────────────────────────────┘
                               │ UI 回调事件 (Callbacks)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 Rust 视图路由层 (Handlers)                   │
│   (file_handlers, session_handlers, host_handlers, etc.)    │
└──────────────────────────────┬──────────────────────────────┘
                               │ 统一读写上下文 (AppContext)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                领域模型与核心状态 (smagical-core)             │
│    (CoreState, Storage, EventManager, TerminalSessionInfo)   │
└─────────────────────────────────────────────────────────────┘
```
