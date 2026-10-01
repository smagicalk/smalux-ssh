# smagical-ui-view

`smagical-ui-view` 是 **smalux-ssh** 的声明式界面与强类型 Slint 生成代码库。它集中托管整个应用所有的 `.slint` 声明式 DSL 文件、统一静态资源（矢量图标、JetBrains Mono 字体、系统托盘图标）以及通过 `slint-build` 宏编译展开生成的全部 Rust 强类型结构与 Handle。

该 crate **不包含后端操作系统调用与业务逻辑**，专注于纯粹的界面展示、组件封装、自适应响应式布局与领域单例桥接（Domain Bridge Singletons）。

---

## 📁 目录结构与架构全景 (Feature-First)

```text
crates/smagical-ui-view/
├── Cargo.toml                  # 依赖清单 (slint, slint-build)
├── build.rs                    # Slint DSL 宏编译脚本 (slint_build::compile_with_config)
├── README.md                   # 模块架构与 UI 组件规范 (本文档)
├── src/
    └── lib.rs                  # include_modules!() 宏展开与字体/托盘静态字节导出
└── ui/
    ├── main.slint              # 全局主窗口总装与根视口路由器
    ├── assets/                 # 统一设计规范图标 (SVG/PNG) 与字体 (TTF)
    ├── themes/                 # 全局设计规范令牌 (Design Tokens) 与预设配色
    │   ├── app-theme.slint     # AppTheme 单例：调色板、圆角、字号、间距与阴影
    │   └── presets/            # 15+ 套终端 (terminal/) 与界面 (ui/) TOML 配色预设
    ├── shared/                 # 🧱 全工程通用共享组件库 (严禁内嵌业务逻辑)
    │   ├── base/               # 原子控件库 (Button, Input, Switch, Dropdown, Slider 等 20+ 基础组件)
    │   ├── composite/          # 复合控件库 (ContextMenu 智能避让右键菜单, SplitPane, TabStrip, DataTable)
    │   └── scaffolds/          # 视图脚手架 (Modal 弹窗容器, MasterDetail 主从布局, Page 页面骨架)
    ├── features/               # 📦 按领域内聚的特性包 (包含专属 Bridge 单例、专属弹窗与表单)
    │   ├── hosts/              # 主机资产 (HostsBridge, 新建分组弹窗, 树选择器, 详情卡片)
    │   ├── terminal/           # 终端视口 (TerminalBridge, 启动器弹窗, 快速连接栏, 查找栏)
    │   ├── settings/           # 系统设置 (SettingsBridge, 8 大独立偏好分类, 备份/导入弹窗)
    │   ├── tunnels/            # 网络隧道 (TunnelsBridge, 隧道配置表单, 跳板选择弹窗)
    │   ├── credentials/        # 安全凭据 (凭据表单, 密钥生成与密码生成弹窗)
    │   ├── snippets/           # 脚本片段 (片段表单, 参数化动态填参弹窗)
    │   ├── file_manager/       # 文件浏览 (双盘容器, 单盘 Browser, 传输队列抽屉)
    │   ├── ai/                 # AI 辅助助手 (AiBridge, 流式对话侧栏)
    │   ├── monitor/            # 资源监控 (MonitorBridge, 实时负载图表)
    │   ├── debug/              # 开发者诊断 (DebugBridge, 日志流与场景模拟)
    │   └── window/             # 桌面窗口控制 (WindowBridge, 最大化/最小化/关闭/拖拽)
    └── views/                  # 顶层视图与侧边抽屉面板
        ├── left_activity_bar.slint    # 左侧 48px 垂直活动栏
        ├── right_tool_bar.slint       # 右侧 48px 辅助工具栏
        ├── center_terminal/           # 中央终端容器 (TabBar, Viewport, StatusBar, FindBar)
        ├── left_drawers/              # 左侧折叠抽屉群 (主机树, 文件树, 历史记录, 凭据, 隧道等)
        ├── right_drawers/             # 右侧折叠抽屉群 (AI 对话, 监控仪表盘, SFTP 传输, 片段库)
        ├── file_explorer_view.slint   # 双盘文件浏览器主视图
        ├── settings_view.slint        # 设置中心主视图
        ├── tunnels_view.slint         # 隧道工作台主视图
        ├── credentials_view.slint     # 凭据保险箱主视图
        ├── snippets_view.slint        # 代码片段工坊主视图
        └── history_center_view.slint  # 连接足迹审计中心主视图
```

---

## 🧩 核心架构模式与设计亮点

### 1. 领域桥接单例 (Slint Domain Bridge Singletons)

为了彻底解决 Slint DSL 中常见的“跨层属性传递爆炸 (Props Drilling)”问题，本项目各领域特性均定义了独立的 `global Bridge`：

- **`HostsBridge`**：维护主机搜索词、分组折叠集、当前选中主机、拖拽状态；
- **`TerminalBridge`**：维护当前活跃 Tab、分屏拓扑索引、字体大小、全屏状态；
- **`SettingsBridge`**：维护当前设置分类（General, Appearance, Terminal, Hotkeys, Security, AI, Sync）、主题切换回调；
- **`TunnelsBridge`**：维护当前隧道列表、启动/停止触发回调、度量更新；
- **`AiBridge`**：维护对话消息流、当前思考状态、指令采纳回调；
- **`WindowBridge`**：维护无边框窗口缩放、窗口标题、关闭至托盘行为；
- **`DebugBridge`**：维护系统实时日志流与测试预设注入。

**优势**：
- **Slint 内部**：深层嵌套的子组件可直接 `HostsBridge.search-filter` 读写，主布局 `main.slint` 无需穿透声明成百上千个属性；
- **Rust 后端**：上层业务 Handlers 可直接通过 `window.global::<HostsBridge>().on_xxx(...)` 挂载事件，完全实现组件级解耦。

---

### 2. 共享组件库体系 (`ui/shared`)

严禁共享组件包含具体业务逻辑，保证组件的高复用性与视觉一致性：

- **原子控件 (`ui/shared/base/`)**：
  - `app-button`：支持 Primary / Secondary / Danger / Ghost 变体，内置 Loading 转圈动效与禁用态；
  - `app-form-input` / `app-password-input`：自适应焦点高亮边框、一键清空按钮、密码显隐切换；
  - `app-dropdown` / `app-segmented-control`：平滑展开下拉项与分段单选控制器；
  - `app-tree-node`：专门针对超深层级优化的树节点原子，内置缩进与平滑展开动画。
- **复合控件 (`ui/shared/composite/`)**：
  - `app-context-menu`：**智能视口边界感知**的右键上下文菜单，自动计算屏幕边缘避免溢出裁剪；
  - `app-split-pane`：支持左右/上下自由拖拽调整比例的分隔面板；
  - `app-data-table`：轻量级表头排序与交替斑马纹数据表格。
- **布局脚手架 (`ui/shared/scaffolds/`)**：
  - `app-modal-scaffold`：具有半透明遮罩、精致居中卡片阴影、ESC 快捷关闭的通用弹窗容器；
  - `app-master-detail-scaffold`：左右主从详情布局脚手架。

---

### 3. 全局设计规范与主题令牌 (`ui/themes/app-theme.slint`)

所有控件的色值、内边距、字号、圆角均引用 `AppTheme` 声明的令牌：

```slint
export global AppTheme {
    // 调色板令牌 (运行时可由后端整体动态注入刷新)
    in-out property <color> background-base: #1e1e2e;
    in-out property <color> background-surface: #181825;
    in-out property <color> border-subtle: #313244;
    in-out property <color> text-primary: #cdd6f4;
    in-out property <color> brand-primary: #89b4fa;
    // 尺寸规范令牌
    out property <length> radius-sm: 4px;
    out property <length> radius-md: 8px;
    out property <length> space-md: 12px;
}
```

---

## 🛠️ 构建与编译流程

1. 本 crate 配备了独立的 `build.rs`，利用 `slint-build` 监听 `.slint` 变动并编译为强类型 Rust 代码；
2. 构建结果通过 `src/lib.rs` 的 `slint::include_modules!()` 导出；
3. 上层 `smagical-ui` 只需在 `Cargo.toml` 中引入 `smagical-ui-view`，即可直接使用生成的强类型窗口类 `AppWindow` 与所有 Bridge。

---

## 🎨 国际化与翻译 (i18n)

- 界面文案统一使用 Slint `@tr(...)` 国际化宏包裹；
- 预留 `translations/` 目录支持多语言 `.po` 字典包打包嵌入。
