# smagical-ui / ui

本目录包含 **Smalux SSH** 客户端的全部 [Slint](https://slint.dev/) 声明式界面代码、原子组件、业务视图、主题配置及静态矢量资源。

---

## 📂 目录架构

```text
crates/smagical-ui/ui/
├── main.slint                  # 顶层主窗口组件 (AppWindow: 组织全屏无边框布局与状态连接)
├── assets/                     # 静态资源目录
│   └── icons/                  # 统一风格的 SVG 矢量图标库 (20+ 矢量图标)
├── components/                 # 高复用原子 UI 组件库
│   ├── action-bar-item.slint   # 活动栏/工具栏图标项组件 (含悬停态、激活条与 Tooltip)
│   ├── command-palette.slint   # Ctrl+K 全局指令与快捷搜索浮层面板
│   ├── create-group-modal.slint# 新建主机分组模态弹窗 (集成树状选择与满宽深色输入框)
│   ├── drawer-container.slint  # 统一可折叠侧边抽屉容器 (含标题、折叠按钮与分割线)
│   ├── group-tree-selector.slint # 通用树状分组选择器 (支持无限层级折叠/展开与单选指示)
│   ├── progress-bar.slint      # 细粒度进度条组件
│   ├── search-input.slint      # 统一暗色搜索输入框 (支持一键清空与原生光标聚焦)
│   ├── status-dot.slint        # 主机健康状态圆点 (在线绿/告警黄/异常红/离线灰)
│   └── tab-item.slint          # 终端标签页组件 (支持标题、状态灯与关闭按钮)
├── themes/                     # 主题系统配置
│   ├── README.md               # 主题 API 与规范说明
│   ├── app-theme.slint         # 全局 Slint 样式单例 (AppTheme: 颜色、圆角、间距令牌)
│   └── presets/                # 15+ 套内置 UI 与 Terminal 配色预设 (TOML)
└── views/                      # 业务视口与功能抽屉
    ├── left_activity_bar.slint # 左侧主活动图标栏 (48px，含日/夜间深浅色切换)
    ├── left_drawers/           # 左侧可折叠功能抽屉群 (240px)
    │   ├── hosts_drawer.slint        # 主机管理抽屉 (树形/卡片双模式 + 横向平滑滚动)
    │   ├── history_drawer.slint      # 历史连接会话列表
    │   ├── files_drawer.slint        # 本地/远程快速文件书签
    │   ├── credentials_drawer.slint  # 账号、密码与 SSH 密钥保管箱
    │   ├── snippets_drawer.slint     # 常用运维脚本与命令片段
    │   ├── tunnels_drawer.slint      # SSH 端口转发与网络隧道
    │   ├── backup_drawer.slint       # 本地配置导入导出与自动备份
    │   └── settings_drawer.slint     # 主题切换与应用偏好设置
    ├── center_terminal/        # 中央终端主视口
    │   ├── tab_bar.slint             # 顶部全宽多标签控制栏 (无边框窗口按钮、广播模式开关)
    │   ├── terminal_viewport.slint   # 终端画布与渲染主区域
    │   └── status_bar.slint          # 底部状态栏 (连接状态、字符编码、网络延迟、主题)
    ├── right_tool_bar.slint    # 右侧辅助工具图标栏 (48px)
    └── right_drawers/          # 右侧辅助工具抽屉群 (240px)
        ├── monitor_drawer.slint      # 远程主机 CPU/内存/磁盘实时资源监控
        ├── snippet_tool_drawer.slint # 终端侧边快捷指令即时发送
        ├── sftp_tool_drawer.slint    # SFTP 文件浏览与上传下载
        ├── notes_tool_drawer.slint   # 运维工作备忘笔记
        └── ai_tool_drawer.slint      # AI 运维助手对话面板
```

---

## 🧩 核心组件与交互规范

### 1. 通用树状分组选择器 (`GroupTreeSelector`)
- **文件路径**：[`components/group-tree-selector.slint`](components/group-tree-selector.slint)
- **设计目的**：在新建分组弹窗、资产迁移等场景中，提供统一、清晰的树形单选体验；
- **交互特性**：
  - 左侧独立 `18px` 折叠三角热区，支持点击折叠/展开；
  - 整行双击快捷折叠/展开子节点；
  - 选中行具备强调色圆点与高亮外框；
  - 内置 `Flickable` 平滑滚动与自隐藏 `3px` 悬浮滚动条。

### 2. 新建主机分组弹窗 (`CreateGroupModal`)
- **文件路径**：[`components/create-group-modal.slint`](components/create-group-modal.slint)
- **设计目的**：为主机资产创建多级层级分组；
- **交互特性**：
  - 居中 `460px x 420px` 精致深色卡片，内容 `100%` 满宽排布无空白；
  - 上半部内嵌 `200px` 高度树形所属上级选择器；
  - 下半部输入框采用纯暗色设计（绝无白底），左侧固定文件夹图标，右侧绝对锚定 `✕` 一键清空按钮；
  - 输入框支持 `Enter` 回车快速提交创建。

### 3. 主机资产管理抽屉 (`HostsDrawer`)
- **文件路径**：[`views/left_drawers/hosts_drawer.slint`](views/left_drawers/hosts_drawer.slint)
- **交互特性**：
  - **双模式自由切换**：支持**树形层级折叠视图**与**卡片列表平铺视图**；
  - **横向自适应滚动**：超长分组或主机名称自动横向延展，激活底部 `4px` 悬浮主题高亮横向滚动条，支持鼠标拖拽、`Shift + 滚轮` 及触摸板双指轻扫；
  - **实时模糊过滤**：顶部搜索框实时联动过滤主机名、IP、端口及分组。

### 4. 左侧主活动导航栏 (`LeftActivityBar`)
- **文件路径**：[`views/left_activity_bar.slint`](views/left_activity_bar.slint)
- **排列顺序**：
  - 上半区（业务资产）：主机管理 $\to$ 文件管理 $\to$ 凭据保管 $\to$ 指令片段 $\to$ 网络隧道 $\to$ **历史会话**；
  - 下半区（系统工具）：数据备份与恢复 $\to$ 偏好设置 $\to$ **深色/浅色一键切换**。

---

## 📐 Slint 布局与编码规范

1. **消除无意拉伸的纵向空隙**：
   - 在 `VerticalLayout` 中，若不需要子元素等距均分空白，务必声明 `alignment: start`。
2. **文本输入框的焦点与裁切约束**：
   - 包含 `TextInput` 的容器应设置 `clip: true` 防止超长文本溢出；
   - 避免使用同级透明 `TouchArea` 覆盖整个 `TextInput` 导致无法获取原生键盘焦点；若需点击背景聚焦，应在 `TouchArea.clicked` 中显式调用 `input.focus()`。
3. **国际化文本规范**：
   - 所有面向用户的静态文本必须使用 `@tr("...")` 包裹；
   - 运行 `extract-translations.ps1` 脚本同步生成/更新 `messages.po`。
