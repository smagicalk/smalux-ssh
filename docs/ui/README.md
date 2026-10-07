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
| **09** | **代码片段与层级脚本中心** | [09_code_snippets.md](file:///F:/code/rust/smalux-ssh/docs/ui/09_code_snippets.md) | 多层文件夹嵌套树、参数化模板引擎 `{{key:default}}`、全屏中心与双侧边抽屉 |
| **10** | **偏好设置中心与多端云同步** | [10_settings_center.md](file:///F:/code/rust/smalux-ssh/docs/ui/10_settings_center.md) | 8 大分类、外观壁纸轮播、终端排版 (CRT/关键字高亮)、网络代理、S3/WebDAV/Gist 云同步与主密码机制 |
| **11** | **网络隧道、跳板机与出网代理** | [11_network_tunnels.md](file:///F:/code/rust/smalux-ssh/docs/ui/11_network_tunnels.md) | 本地/远程/动态端口转发、多跳跳板机堡垒链路、静态出网代理节点与拓扑速率波形图 |
| **12** | **特性内聚与组件模块化重构指南** | [12_architecture_refactor_guide.md](file:///F:/code/rust/smalux-ssh/docs/ui/12_architecture_refactor_guide.md) | 特性内聚目录结构 (`shared/` + `features/`)、Slint 领域桥接单例 (`Domain Bridge`) 与 4 级原子设计系统 |

---

## 🏗️ 现代工程布局规范 (Microkernel & Plugin Topology)

工程采用 **“通用基础集中 (`crates/ui/common/`) + 统一构建内核 (`crates/ui/kernel/`) + 页面级独立插件 (`crates/ui/plugins/<plugin>/`)”** 现代化微前端拓扑，实现高内聚低耦合：

```text
crates/ui/
├── common/                     # 🧱 跨插件共享设计系统 (严禁依赖任何具体业务插件)
│   ├── ui/themes/              # 设计令牌 AppTheme 与 15+ 套终端/界面主题 TOML 预设
│   ├── ui/shared/base/         # 原子控件 (按钮、输入框、开关、下拉框、分段器)
│   ├── ui/shared/composite/    # 复合控件 (ContextMenuContainer, SplitPane, DataTable)
│   └── ui/shared/scaffolds/    # 脚手架 (AppModalScaffold, AppMasterDetailScaffold, AppFormRow)
├── kernel/                     # 🚀 微内核底座 (全工程单一 Slint build.rs 构建入口)
│   ├── build.rs                # 挂载 @common 与全部 @plugin-* 虚拟库路径
│   └── ui/
│       ├── kernel.slint        # 全局主窗口总装与根视口路由器 (AppWindow)
│       ├── views/              # 终端核心视口 (terminal_viewport)、活动栏 (left_activity_bar)、工具栏
│       └── components/         # 全局弹窗 (new-session-modal, vault_unlock_modal 等)
└── plugins/                    # 📦 8 大页面级独立插件 (与左侧活动栏 8 个图标严格 1:1 对齐)
    ├── hosts/                  # [页面 1: 主机资产] (含 companion/ 独立伴生目录: ai/, monitor/, tmux/)
    ├── files/                  # [页面 2: 文件管理器] (含 companion/ sftp 传输抽屉)
    ├── snippets/               # [页面 3: 代码片段库] (含 companion/ 快速命令抽屉)
    ├── tunnels/                # [页面 4: 端口隧道拓扑] (含 companion/ 快速控制抽屉)
    ├── credentials/            # [页面 5: 凭据保管箱] (密钥管理、指纹解析、密钥生成)
    ├── history/                # [页面 6: 连接审计历史] (时间流审计、终端快照回溯)
    ├── settings/               # [页面 7: 偏好设置外观] (外观工坊、取色器、全屏设置、备份)
    └── debug/                  # [页面 8: 开发者调试台] (状态探针、批量模拟、日志查看器)
```

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
