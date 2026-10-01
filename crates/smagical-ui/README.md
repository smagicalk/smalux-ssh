# smagical-ui

`smagical-ui` 是 **smalux-ssh** 的桌面客户端业务装配与控制中枢 crate。它基于 [Slint UI](https://slint.dev/) 框架与 Tokio 异步运行时构建，负责桌面窗口生命周期管理、**高性能软件光栅化终端渲染引擎 (PTY/Parser/Renderer/SplitTree)**、**1:1 镜像领域 Handlers 处理器集群**、**增量 Diff 树形状态缓存**、**跨平台本地 Shell 探测**、**系统托盘集成**以及**动态主题运行时热注入**。

界面声明与纯 Slint 视图模型由下层的 [`smagical-ui-view`](../smagical-ui-view/README.md) 提供。

---

## 📁 目录结构与模块全景

```text
crates/smagical-ui/
├── Cargo.toml                  # 依赖清单 (slint, tokio, portable-pty, alacritty_terminal, fontdue 等)
├── README.md                   # 模块架构与子模块职责规范文档 (本文档)
└── src/
    ├── main.rs                 # 可执行二进制启动入口 (初始化日志与 Tokio 运行时)
    ├── lib.rs                  # 桌面应用总装中枢 (run: 状态挂载、Handlers 绑定、事件总线监听)
    ├── async_util.rs           # Slint UI 线程与 Tokio 异步运行时通信桥梁 (run_async, run_async_local)
    ├── local_shells.rs         # 跨平台本地 Shell (PowerShell, CMD, Git Bash, WSL, Bash) 环境探测与缓存引擎
    ├── launcher_prewarm.rs     # 快速新建终端启动器 (Launcher) 数据预热与缓存
    ├── pipeline_config.rs      # 存储模式启动管线装配
    ├── storage_config.rs       # 存储后端模式初始化策略
    ├── tree_model.rs           # 主机树纯函数算法层 (RawTreeNode, 扁平化展开, 拖拽重排, 循环成环阻断, 搜索过滤)
    ├── snippet_tree_model.rs   # 代码片段多级目录树纯函数算法层
    ├── session.rs              # 终端活跃会话与 Slint UI 状态双向同步
    ├── debug_ui.rs             # Tracing 日志流与 Slint Debug 面板数据桥接
    ├── tray.rs                 # 跨平台系统托盘生命周期、托盘菜单与气泡通知集成
    ├── tunnel_daemon.rs        # 网络隧道后台转发守护进程
    ├── activity_bar_service.rs # 左侧 48px 活动栏切换与徽章计数服务
    ├── right_panel_service.rs  # 右侧伴生辅助面板 (AI/监控/SFTP/片段) 互斥展开服务
    ├── notification_service.rs # 统一 Toast 消息横幅通知调度引擎
    ├── terminal/               # 🖥️ 高性能终端引擎子系统
    │   ├── mod.rs              # 终端子系统集中导出
    │   ├── instance.rs         # 终端实例管理器 (PTY 读写流调度、选择区高亮、鼠标按键事件处理)
    │   ├── pty.rs              # 基于 portable-pty 的跨平台伪终端进程调度与窗口尺寸同步
    │   ├── parser.rs           # ANSI / VT100 / XTerm 转义序列状态机解析器
    │   ├── renderer.rs         # 基于 fontdue 的纯 CPU 软件光栅化字形着色与网格渲染器 (Slint Image)
    │   ├── split_tree.rs       # 终端分屏二叉树拓扑结构 (水平/垂直切分、焦点轮转、比例自适应)
    │   ├── highlight.rs        # 正则关键词高亮规则引擎 (IP 地址、URL、错误堆栈着色)
    │   ├── key_encoder.rs      # 物理按键与修饰键到 VT 转义序列的精准编码器
    │   └── ssh_config.rs       # 终端 SSH 参数适配器
    ├── handlers/               # 🎮 1:1 领域事件处理器集群 (无污染解耦绑定)
    │   ├── mod.rs              # Handlers 统一挂载装配入口 (attach_all_handlers)
    │   ├── host_handlers.rs    # 主机/分组资产 CRUD、树形拖拽重排与列表排序
    │   ├── session_handlers.rs # 终端 Tab 会话生命周期 (新建、分屏、聚焦、关闭、重连)
    │   ├── credential_handlers.rs # 安全凭据存取、SSH 密钥对生成、密码生成与保险库解锁
    │   ├── tunnel_handlers.rs  # 网络隧道配置、端口占用冲突检测与后台守护进程启闭
    │   ├── file_handlers.rs    # 双盘文件浏览器 IO、跨栏拖拽上传下载、传输任务队列调度
    │   ├── right_drawer_handlers.rs # AI 对话交互、实时负载监控、代码片段快执
    │   ├── window_handlers.rs  # 无边框窗口动作 (拖拽移动、最大化/最小化、关闭至托盘)
    │   ├── theme_handlers.rs   # 动态主题切换、主题设计工坊实时预览与 TOML 导入
    │   ├── debug_handlers.rs   # 调试日志流实时拉取、测试场景预设注入、批量数据模拟
    │   ├── color_utils.rs      # 颜色格式化与十六进制转换工具
    │   └── settings_handlers/  # 偏好设置分类处理器 (ai, backup, highlight, keybinding, security, utils)
    ├── store/                  # 💾 增量 Diff 与 UI 状态缓存层
    │   ├── mod.rs              # 存储缓存导出
    │   ├── host_store.rs       # 主机树内存缓存与快速索引映射表
    │   └── diff.rs             # 树形增量 Diff 算法 (避免全量刷新引起 Slint 界面闪烁)
    ├── theme/                  # 🎨 主题运行时应用子系统
    │   ├── mod.rs              # 主题导出
    │   ├── apply.rs            # 将 Core 层 ThemeDefinition 动态注入 Slint AppTheme 运行时属性
    │   └── builtins.rs         # 静态打包内置的 15+ 套经典主题
    └── debug/                  # 🛠️ 开发者诊断与内存探针
        ├── mod.rs              # 调试导出
        ├── tracing_layer.rs    # 自定义 Tracing Layer，日志捕获至内存环形缓冲区 (CircularBuffer)
        ├── inspector.rs        # 运行时状态探针与内存诊断
        ├── batch.rs            # 批量测试数据生成器
        ├── presets.rs          # 常用网络与服务器模拟场景预设
        ├── logger.rs           # 调试日志格式化
        └── models.rs           # 调试视图数据模型
```

---

## 🧩 各子系统与核心子模块 (`mod`) 详细职责解析

### 1. `terminal` - 高性能纯 Rust 终端渲染引擎

为了提供丝滑、低延迟且跨平台的终端体验，系统自研了软光栅终端引擎，无需依赖 WebView 或外部终端控件：

- **[`terminal::instance`](src/terminal/instance.rs)**：
  - 终端实例的核心中枢，连接 PTY 进程与渲染层；
  - 维护终端选择区状态（鼠标拖选、双击选词、三击选行）、剪贴板文本复制与粘贴；
  - 提供异步 PTY 输出消费循环，触发脏帧标记与 Slint 局部重绘。
- **[`terminal::pty`](src/terminal/pty.rs)**：
  - 基于 `portable-pty` 抽象跨平台伪终端；
  - 在 Windows 下使用 ConPTY 原生伪终端驱动，在 Linux/macOS 下使用 OpenPTY；
  - 监听终端视口尺寸变化，动态同步调整 PTY 行列数 (`pty.resize(rows, cols)`)。
- **[`terminal::parser`](src/terminal/parser.rs)**：
  - 解析 ANSI / VT100 / XTerm 控制序列（光标移动、清屏、SGR 样式属性、备用屏幕切换）；
  - 维护回滚缓冲区（Scrollback Buffer），支持最多 100,000 行历史翻页。
- **[`terminal::renderer`](src/terminal/renderer.rs)**：
  - 基于 `fontdue` 纯 Rust 字体光栅化器，将 JetBrains Mono 等等宽字体字符转换为像素 Alpha 遮罩；
  - 内存级字形缓存（Glyph Cache），相同字符零重复光栅化；
  - 直接在内存中组装 RGBA 像素矩阵并生成 `slint::Image`，实现极高帧率的 CPU 软光栅渲染。
- **[`terminal::split_tree`](src/terminal/split_tree.rs)**：
  - 二叉树分屏数据结构（`SplitTree::Leaf` / `SplitTree::Node`）；
  - 支持无级水平分屏 (`Horizontal`) 与垂直分屏 (`Vertical`)；
  - 支持动态拖拽调整窗格比例、分屏焦点轮转切换（`Ctrl+Alt+方向键`）、单窗格关闭与自动折叠平衡。
- **[`terminal::highlight`](src/terminal/highlight.rs)**：
  - 实时正则匹配引擎：对终端输出流中的 IPv4/IPv6、URL 超链接、时间戳、SQL 关键词、`ERROR` / `WARN` / `FAIL` 进行动态着色增强。
- **[`terminal::key_encoder`](src/terminal/key_encoder.rs)**：
  - 将 Slint 键盘事件（键码、Control/Shift/Alt/Meta 状态）精准转换为终端标准的 VT 转义序列（如 `\x1b[A`、`\x1b[1;5C`）。

---

### 2. `handlers` - 1:1 领域事件处理器集群

通过统一入口 `attach_all_handlers(&window, &core_state, ...)` 挂载所有 UI 回调，全面解耦视图层与底层业务：

- **`host_handlers`**：
  - 监听 `HostsBridge` 树形拖拽信号，执行 `move_and_reorder_raw_node`；
  - 具备**循环成环检测保护**（禁止将父分组拖入其自身的子孙节点内部）；
  - 增量保存至存储层 (`core_state.storage().hosts().save(...)`)。
- **`session_handlers`**：
  - 监听新建 Tab、关闭 Tab、Tab 左右拖拽排序；
  - 启动快速新建终端启动器（Launcher），列出本地所有 Shell 与主机资产，提供毫秒级拼音/模糊匹配。
- **`credential_handlers`**：
  - 生成 Ed25519 / RSA-4096 / ECDSA SSH 密钥对；
  - 生成高熵强密码；
  - 弹出主密码保险库解锁对话框并挂载 DEK。
- **`tunnel_handlers`**：
  - 本地端口/远端端口占用检测；
  - 启动/停止后台隧道守护任务 (`tunnel_daemon`)。
- **`file_handlers`**：
  - 双盘本地磁盘扫描与路径解析；
  - 跨栏拖拽上传/下载文件事件拦截；
  - 多层级传输任务树进度同步。
- **`right_drawer_handlers`**：
  - 流式 AI 对话请求与 Markdown 渲染；
  - 服务器 CPU/内存实时监控图表更新；
  - 代码片段模板参数提取弹窗与执行注入。
- **`settings_handlers`**：
  - 8 大分类偏好读写、主题切换、全量数据导出加密备份与恢复。

---

### 3. `store` - 增量 Diff 与 UI 状态缓存

- **[`store::diff`](src/store/diff.rs)**：
  - 实现了基于唯一 ID 的列表增量 Diff 计算算法；
  - 对比新旧数据树，仅对发生增、删、改或位置变动的节点触发 Slint Model 局部更新，彻底消除全量重刷造成的视觉闪烁与光标重置。
- **[`store::host_store`](src/store/host_store.rs)**：
  - 缓存扁平化与树形节点索引，优化搜索与过滤速度。

---

### 4. `theme` - 运行时主题热注入

- **[`theme::apply`](src/theme/apply.rs)**：
  - 接收 Core 层解析好的 `ThemeDefinition`；
  - 将数十项颜色 Token（背景色、前景色、强调色、卡片边框、终端 ANSI 16 色）动态注入到 Slint 的 `AppTheme` 全局单例属性中；
  - 支持无需重启应用实现即时平滑换肤。

---

### 5. `debug` - 开发者诊断与 Tracing 环形缓冲

- **[`debug::tracing_layer`](src/debug/tracing_layer.rs)**：
  - 实现自定义 `tracing_subscriber::Layer`；
  - 将全工程产生的 `trace`, `debug`, `info`, `warn`, `error` 日志实时压入带容量限制的内存环形缓冲区（`CircularBuffer`）；
  - UI 调试抽屉直接从该缓冲区拉取日志，极大方便现场排错。

---

### 6. 顶层服务组件

- **`local_shells.rs`**：跨平台（Windows / Linux / macOS）探测已安装的 PowerShell 7/5.1、CMD、Git Bash、WSL 实例、Bash，并启动时单次探测缓存；
- **`tray.rs`**：系统托盘常驻，支持关闭主窗口时最小化至托盘、双击托盘图标快速唤醒、托盘右键快捷断开所有会话；
- **`async_util.rs`**：封装 `run_async` 与 `run_async_local`，妥善处理 Tokio 异步任务与 Slint UI 线程（`slint::invoke_from_event_loop`）之间的数据通信，杜绝界面卡顿与死锁。

---

## 🛠️ 常用开发命令

```bash
# 启动桌面客户端
cargo run -p smagical-ui

# 静态代码检查 (严格 0 警告)
cargo clippy -p smagical-ui --all-targets -- -D warnings

# 单元测试 (62+ 项 UI 纯函数、Diff 算法、终端解析与主题测试)
cargo test -p smagical-ui
```
