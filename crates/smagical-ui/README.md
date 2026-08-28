# smagical-ui

`smagical-ui` 是 `smalux-ssh` 的桌面客户端展示层 crate，基于 [Slint UI](https://slint.dev/) 框架构建，负责桌面窗口生命周期管理、UI 视图布局、交互回调绑定、主题动态应用以及多语言国际化支持。

---

## 📁 目录结构

```text
crates/smagical-ui/
├── Cargo.toml                  # UI crate 依赖与配置
├── build.rs                    # Slint 编译构建脚本 (slint_build::compile)
├── extract-translations.ps1    # i18n 提取脚本
├── messages.po                 # gettext 多语言文案目录
├── src/
│   ├── lib.rs                  # 桌面应用入口、事件总线与核心服务装配
│   ├── main.rs                 # 可执行二进制入口
│   └── theme/                  # Slint 主题注册、内置资源加载与动态应用
└── ui/
    ├── main.slint              # 顶层主窗口组件 (AppWindow)
    ├── assets/                 # 图标与静态 SVG 矢量图片资源
    ├── components/             # 通用基础 UI 组件库
    │   ├── action-bar-item.slint     # 工具栏/活动栏图标项
    │   ├── command-palette.slint     # Ctrl+K 全局指令面板
    │   ├── create-group-modal.slint  # 新建主机分组模态对话框 (支持层级选择与极简暗色输入)
    │   ├── drawer-container.slint    # 统一折叠抽屉容器
    │   ├── group-tree-selector.slint # 通用树状分组选择器 (支持多级折叠/展开与单选指示)
    │   ├── progress-bar.slint        # 进度指示条
    │   ├── search-input.slint        # 搜索输入框
    │   ├── status-dot.slint          # 连接/健康状态圆点
    │   └── tab-item.slint            # 终端标签页组件
    ├── themes/                 # 主题系统与内置配色预设
    │   ├── README.md           # 主题规范与 API 说明文档
    │   ├── app-theme.slint     # 全局 Slint 主题单例 (AppTheme / AppColorScheme)
    │   └── presets/            # 15+ 套 UI 及 Terminal 配色预设 (TOML)
    │       ├── ui/             # UI 主题 (Darcula, Catppuccin, Monokai, Nord 等)
    │       └── terminal/       # 终端 ANSI 16 色主题
    └── views/                  # 各区域业务视图组件
        ├── left_activity_bar.slint   # 左侧主活动图标栏 (48px，支持日/夜间一键切换)
        ├── left_drawers/             # 左侧可折叠功能抽屉 (240px)
        │   ├── hosts_drawer.slint        # 主机资产树与连接管理 (树形/卡片双模式 + 横向平滑滚动)
        │   ├── history_drawer.slint      # 连接历史记录
        │   ├── files_drawer.slint        # 本地/远程快速文件书签
        │   ├── credentials_drawer.slint  # 凭据与密钥管理
        │   ├── snippets_drawer.slint     # 常用脚本/命令片段
        │   ├── tunnels_drawer.slint      # SSH 端口转发与隧道
        │   ├── backup_drawer.slint       # 配置备份与恢复
        │   └── settings_drawer.slint     # 外观主题与系统设置
        ├── center_terminal/          # 中央终端与控制主区域
        │   ├── tab_bar.slint             # 顶部全宽控制栏 (标签页、搜索、广播、无边框控制)
        │   ├── terminal_viewport.slint   # 终端视口主区域
        │   └── status_bar.slint          # 底部状态栏
        ├── right_tool_bar.slint      # 右侧辅助工具栏 (48px)
        └── right_drawers/            # 右侧辅助功能抽屉 (240px)
            ├── monitor_drawer.slint      # 实时系统负载与硬件监控
            ├── snippet_tool_drawer.slint # 终端快捷命令发送抽屉
            ├── sftp_tool_drawer.slint    # SFTP 文件浏览与传输
            ├── notes_tool_drawer.slint   # 快速备忘与运维笔记
            └── ai_tool_drawer.slint      # AI 运维辅助助手
```

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
| 栏 | 密钥 / 脚本|                                                    | 笔记 / AI  | 栏 |
|    | 隧道 / 历史|                                                    |            |    |
|48px| (可折叠)   |                                                    | (可折叠)   |48px|
+----+------------+----------------------------------------------------+------------+----+
| StatusBar: 状态: 已连接 (127.0.0.1:22) | UTF-8 | 延迟: 12ms | 主题: Darcula       |
+-----------------------------------------------------------------------------------+
```

### 核心交互与组件机制

1. **🌲 树形层级管理与多模式视图 (`HostsDrawer`)**：
   - 支持**树形层级模式**与**卡片列表模式**一键切换；
   - 树形模式下支持级联折叠/展开，超长节点名称自适应横向展开，底部配备自隐藏超薄横向滚动条（支持鼠标拖拽、`Shift + 滚轮` 及触摸板双指轻扫）；
   - 支持在搜索框内进行多维度实时模糊过滤（按主机名、IP、分组）。
2. **📂 独立树状分组选择器 (`GroupTreeSelector`)**：
   - 具备独立的折叠/展开指示三角点击热区，支持点击与双击切换；
   - 单选圆圈指示与高亮边框联动。
3. **✨ 现代化新建分组弹窗 (`CreateGroupModal`)**：
   - 460x420px 居中卡片容器，内容 100% 满宽排布；
   - 内置 200px 树状上级分组选择器；
   - 纯暗色输入框（左侧固定图标，右侧绝对锚定一键清空按钮），支持回车直接提交。
4. **🧭 左侧主活动栏导航 (`LeftActivityBar`)**：
   - 调整优化业务图标流（主机管理 $\to$ 文件管理 $\to$ 凭据保管 $\to$ 指令片段 $\to$ 网络隧道 $\to$ 历史会话）；
   - 底部提供经典系统维护区（备份导入、偏好设置、深色/浅色一键切换）。
5. **⌨️ 全局指令面板 (`CommandPalette`)**：
   - 快捷键 `Ctrl + K` 或 `Ctrl + P` 呼出浮动搜索面板，支持快速切换主机、执行命令和更换主题。
6. **🪟 无边框窗口控制与多端广播**：
   - 自定义标题栏按钮（最小化、最大化/还原、关闭）；
   - 支持一键开启广播模式向所有打开的终端分发键盘输入。

---

## 🎨 主题系统集成

UI 样式通过 `AppTheme` 单例统一定义，颜色与尺寸令牌与 `smagical-core::theme` 模型严格对应：

- **丰富预设**：内置 15+ 套经典浅色/深色主题（`Darcula`, `Catppuccin`, `Monokai`, `Nord`, `One Dark`, `Solarized`, `Tokyo Night`, `Rosé Pine` 等）；
- **平滑换肤**：在 Rust 端通过 `apply_theme_by_id(&window, &themes, theme_id)` 动态将解析后的色值推送到 Slint 运行时属性；
- 详细规范请参阅 [ui/themes/README.md](ui/themes/README.md)。

---

## 🌐 国际化 (i18n)

UI 字符串统一使用 Slint 的 `@tr(...)` 宏包裹，实现界面文案的提取与本地化：

- **文案目录**：[`messages.po`](messages.po)
- **提取工具**：`slint-tr-extractor` (v1.16.1)
- **提取脚本**：
  ```powershell
  & 'crates/smagical-ui/extract-translations.ps1'
  ```

---

## 🛠️ 常用开发命令

### 运行应用
```bash
cargo run -p smagical-ui
```

### 代码检查与单元测试
```bash
cargo check -p smagical-ui
cargo test -p smagical-ui
```
