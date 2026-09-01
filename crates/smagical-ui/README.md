# smagical-ui

`smagical-ui` 是 **smalux-ssh** 的桌面客户端展示与交互层 crate，基于 [Slint UI](https://slint.dev/) 框架构建。它负责桌面窗口生命周期、UI 视图布局渲染、交互回调路由、本地 Shell 探测、主题动态注入以及多语言国际化支持。

---

## 📁 目录与模块架构

```text
crates/smagical-ui/
├── Cargo.toml                  # UI crate 依赖与配置
├── build.rs                    # Slint 编译构建脚本 (slint_build::compile)
├── extract-translations.ps1    # i18n 提取脚本
├── messages.po                 # gettext 多语言文案目录
├── src/
│   ├── lib.rs                  # 桌面应用入口 (run) 与 Slint 全局回调路由
│   ├── main.rs                 # 可执行二进制启动入口
│   ├── tree_model.rs           # 树形视图纯函数操作层 (RawTreeNode, 排序, 拖拽迁移, 搜索过滤)
│   ├── session.rs              # 终端会话管理与 Slint UI 状态同步
│   ├── debug_ui.rs             # Tracing 日志面板数据桥接
│   ├── local_shells.rs         # 跨平台本地 Shell 环境探测与缓存引擎
│   ├── right_panel_service.rs  # 右侧工具抽屉服务
│   ├── handlers/               # 领域交互回调分离模块
│   │   ├── mod.rs              # AppContext 共享上下文定义
│   │   ├── file_handlers.rs    # 双盘文件管理与 SFTP 传输交互处理器
│   │   ├── history_handlers.rs # 历史中心交互处理器
│   │   ├── host_handlers.rs    # 主机管理交互处理器
│   │   ├── session_handlers.rs # 终端会话交互处理器
│   │   └── window_handlers.rs  # 窗口生命周期与抽屉交互处理器
│   └── theme/                  # Slint 主题注册、内置资源加载与动态应用
└── ui/
    ├── main.slint              # 顶层主窗口组件 (AppWindow)
    ├── assets/                 # 统一风格 SVG 矢量图标库
    ├── components/             # 通用基础原子 UI 组件库 (context-menu, modals, tree)
    ├── themes/                 # 主题样式规范与 TOML 预设配置
    └── views/                  # 各区域业务视图组件 (file_explorer_view, history, terminal)
```

---

## 🧩 Rust 端核心模块解耦设计

| 模块 | 核心职责 | 特性与设计说明 |
| :--- | :--- | :--- |
| [`lib.rs`](src/lib.rs) | 组装层与事件路由 | 负责 Slint 窗口实例化、状态机装配与 UI 事件分发，代码保持高度轻量与可读。 |
| [`tree_model.rs`](src/tree_model.rs) | 树形纯函数模型 | 定义 UI 专用内部模型 `RawTreeNode`。包含 `build_raw_tree_from_storage`（直属子项计数）、`move_and_reorder_raw_node`（防循环引用与四模式调序）、`build_visible_tree_nodes` 等无副作用纯函数。 |
| [`session.rs`](src/session.rs) | 会话与 Tab 状态同步 | 管理 `TerminalSessionInfo`，负责多终端 Tab 的创建、激活、切换与关闭状态同步。 |
| [`handlers/file_handlers.rs`](src/handlers/file_handlers.rs) | 双盘文件与传输调度 | 负责双栏 Tab 调度、本地磁盘遍历与文件操作、轻量 Tab 重排同步 (`sync_local_tabs_only` / `sync_remote_tabs_only`)、右键上下文操作与拖拽传输任务构建。 |
| [`debug_ui.rs`](src/debug_ui.rs) | 调试日志同步 | 桥接 `smagical-debug` 内存环形缓冲区至 Slint 调试抽屉模型。 |
| [`local_shells.rs`](src/local_shells.rs) | 跨平台本地 Shell 探测 | 启动时一次性扫描系统中的 PowerShell、WSL、Git Bash、CMD、Bash、Zsh 等终端并全局缓存，杜绝重复磁盘 I/O。 |

---

## 🖥️ 界面架构与布局

主窗口采用无边框现代化设计，由 **`ui/main.slint`** 统领全局：

```text
+-----------------------------------------------------------------------------------+
| TabBar: [标签 1] [标签 2] [+]   [快捷搜索 Ctrl+K]   [广播] [换肤] [设置] [_] [□] [✕] |
+----+------------+----------------------------------------------------+------------+----+
|    |            |                                                    |            |    |
| 左 |  左侧抽屉  |                                                    |  右侧抽屉  | 右 |
| 侧 | (240px)    |                                                    | (240px)    | 侧 |
| 活 |            |                     中央终端主视口                 |            | 工 |
| 动 | 主机 / 文件|                   (TerminalViewport)               | 监控 / SFTP| 具 |
| 栏 | 密钥 / 脚本|                                                    | 调试 / 笔记| 栏 |
|    | 隧道 / 历史|                                                    |            |    |
|48px| (可折叠)   |                                                    | (可折叠)   |48px|
+----+------------+----------------------------------------------------+------------+----+
| StatusBar: 状态: 已连接 (127.0.0.1:22) | UTF-8 | 延迟: 12ms | 主题: Darcula       |
+-----------------------------------------------------------------------------------+
```

### 核心交互特性

1. **🌲 主机资产双视图模式 (`HostsDrawer`)**：
   - **树形层级模式**：支持无限层级拖拽调序（Before / After / Inside）、循环引用阻断保护、超宽节点横向平滑滚动；
   - **卡片列表模式**：平铺大卡片展示，支持独立拖拽视觉排序（锁定分组属性不变）。
2. **📂 双盘文件浏览器与传输工作台 (`FileExplorerView`)**：
   - **独立双栏 Tab 架构**：左栏本地文件系统与右栏远程 SFTP 拥有独立的 Tab 栈、双向历史（后退/前进/上级）与路径输入跳转；
   - **同栏 Tab 丝滑拖拽调序**：支持左键按住 Tab 左右拖拽重排顺序；跨栏或拖出 Tab 栏自动显示 `🚫 禁止` 置灰反馈与安全复位；
   - **跨栏拖拽文件传输**：支持本地向远程拖拽上传、远程向本地拖拽下载；同窗口内拖拽自动判定无效；
   - **多层级传输任务树**：支持多文件与多层级文件夹递归拆解传输任务树，默认折叠汇总显示总进度与传输速率；
   - **智能右键菜单 (`ContextMenuContainer` & `ContextMenuItem`)**：统一定义 4 大右键菜单（文件、传输、终端视口、终端 Tab），支持智能视口避让翻转、100% 实体高对比度分割线与即时响应消失。
3. **📂 独立树状分组选择器 (`GroupTreeSelector`)**：
   - 具备独立折叠三角热区与双击快捷展开，单选圆圈指示与高亮联动。
4. **✨ 现代化新建分组弹窗 (`CreateGroupModal`)**：
   - 460x420px 居中精致卡片，内嵌树状上级选择器与纯暗色一键清空输入框。
5. **🐚 快速新建终端弹窗 (Launcher Modal)**：
   - 动态列出本地所有可用 Shell 环境与远程主机资产，支持毫秒级拼音/关键字实时模糊过滤。
6. **🛠️ 开发者调试抽屉 (`DebugDrawer`)**：
   - 查看全系统实时 Tracing 日志流，支持一键注入场景预设（K8s/微服务/压测）、批量生成主机资产及端口状态模拟。

---

## 🎨 主题系统集成

UI 样式通过 `AppTheme` 单例统一定义，颜色与尺寸令牌与 `smagical-core::theme` 模型严格对应：

- **丰富预设**：内置 15+ 套经典浅色/深色主题（`Darcula`, `Catppuccin`, `Monokai`, `Nord`, `One Dark`, `Solarized`, `Tokyo Night`, `Rosé Pine` 等）；
- **平滑换肤**：在 Rust 端通过 `apply_theme_by_id(&window, &themes, theme_id)` 动态将解析后的色值推送到 Slint 运行时属性；
- 详细规范请参阅 [ui/themes/README.md](ui/themes/README.md)。

---

## 🌐 国际化 (i18n)

UI 字符串统一使用 Slint 的 `@tr(...)` 宏包裹：

- **文案目录**：[`messages.po`](messages.po)
- **提取工具**：`slint-tr-extractor` (v1.16.1)
- **提取脚本**：
  ```powershell
  & 'crates/smagical-ui/extract-translations.ps1'
  ```

---

## 🛠️ 常用开发命令

```bash
# 启动应用
cargo run -p smagical-ui

# 静态检查 (0 警告)
cargo clippy -p smagical-ui --all-targets -- -D warnings

# 单元测试 (8 项 UI 纯函数与主题测试)
cargo test -p smagical-ui
```
