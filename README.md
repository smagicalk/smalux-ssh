# smagicalssh

Rust 跨平台桌面 SSH 工作台，目标是参考并整合 XTerminal、Termora、Termius 的核心能力，提供一个以终端为中心的连接管理与远程运维工具。

## 项目定位

- 面向开发者和运维人员
- 跨平台桌面应用
- 以 SSH 为核心，扩展到 SFTP、端口转发、隧道、工作区和资产管理
- 优先采用纯 Rust 技术栈

## 需求范围

### SSH 与终端

- 交互式 shell / PTY
- 远程命令执行
- 多标签页
- 分屏
- 工作区恢复
- 终端搜索、复制粘贴、滚动缓冲、快捷键
- 256 色、真彩色、鼠标事件支持

### 认证与安全

- 密码
- 私钥
- 私钥口令
- `ssh-agent`
- OpenSSH certificate
- Known Hosts 校验和管理
- 本地安全凭据存储

### 主机资产管理

- 主机保存
- 分组
- 标签
- 最近连接
- 快速搜索
- OpenSSH config 导入 / 导出
- 跳板机 / `ProxyJump`
- 多级跳板
- SOCKS5 / HTTP 代理

### SFTP

- 图形化文件管理
- 上传 / 下载
- 拖拽
- 批量传输
- 远程编辑
- 权限编辑
- 书签
- 与终端当前目录联动
- 目录同步

### 端口转发 / 隧道

- Local Forwarding
- Remote Forwarding
- Dynamic SOCKS5
- 规则保存
- 启动 / 停止 / 状态监控
- 自动重连

### 效率增强

- 命令历史
- Snippets / 快捷命令
- 动态变量
- 参数历史
- 启动命令
- 批量执行

### 本地终端

- PowerShell
- CMD
- WSL
- Bash / Zsh / Fish

### 日志与监控

- 终端日志
- 会话录制
- 回放
- 操作审计日志
- CPU / 内存 / 磁盘 / 网络 / 延迟 / 系统信息面板

### 同步与扩展

- 本地加密存储
- 后续支持 WebDAV / Gist / 自建同步
- 插件化扩展
- Telnet
- Serial
- Zmodem
- RDP
- VNC
- FTP / S3 / WebDAV / SMB

### AI 能力

- 命令解释
- 报错诊断
- 命令生成
- 外部 API 可配置

## 界面与视觉

### 主题

- 全局主题
- 主机级主题覆盖
- 明暗切换
- 配色可配置
- 字体、字号、终端样式独立配置

### 背景

- 全页面背景生效
- 背景图片轮转
- 图片透明度可调
- 图片模糊度可调
- 支持本地图片源
- 支持在线图片源

### 布局参考

- 左侧：Hosts、Groups、Recent、Snippets、Tunnels、Keys、Settings
- 中间：主工作区，终端标签页与分屏
- 右侧：当前连接信息、SFTP、端口转发、命令、日志
- 顶部或底部：快速连接、搜索、命令面板

## 技术栈

- GUI: `iced`
- 异步运行时: `tokio`
- SSH: `russh`
- SFTP: `russh-sftp`
- 终端状态机: `alacritty_terminal`
- 本地 PTY: `portable-pty`
- 配置序列化: `serde`
- 配置文件格式: `toml` 或 `ron`
- 本地数据库: `redb` 或 `sqlite`
- 凭据存储: `keyring`
- 日志: `tracing`
- 目录与路径: `directories`

## 实现原则

- 优先纯 Rust
- 终端体验优先
- 配置分为全局默认与主机覆盖两级
- 视觉层与终端层解耦
- 不做像素级仿制，做能力对标与体验整合

## 架构草案

### 分层

- 表现层: `iced` 负责窗口、布局、事件、主题、背景与各类面板
- 领域层: 主机、会话、分组、隧道、Snippets、主题、背景、日志等核心数据模型
- 服务层: SSH、SFTP、端口转发、PTY、配置存储、凭据存储、同步
- 平台适配层: `ssh-agent`、本地文件选择、系统凭据库、在线图片源、本地终端入口

### 核心模块

- `app-shell`: 窗口壳、导航、标签页、分屏、命令面板
- `session-core`: SSH 连接生命周期、认证、KeepAlive、重连
- `terminal-core`: 终端状态机、输入输出、滚动缓冲、搜索
- `sftp-core`: 文件浏览、传输队列、远程编辑、权限操作
- `forward-core`: Local / Remote / Dynamic 隧道与规则管理
- `storage-core`: 主机、分组、标签、最近连接、主题、背景、Snippets 持久化
- `security-core`: 密码、密钥、证书、Known Hosts、凭据引用
- `visual-core`: 主题系统、背景轮转、透明度、模糊度、图片源管理

### 配置模型

- 全局配置: 主题、背景、字体、默认连接参数、窗口行为
- 主机配置: 认证方式、跳板机、代理、主题覆盖、背景覆盖、隧道规则
- 会话配置: 标签页状态、分屏状态、最近目录、命令历史
- 视觉配置: 背景播放列表、图片透明度、模糊度、切换间隔、来源类型

### 主要数据对象

- `Host`
- `HostGroup`
- `ConnectionProfile`
- `AuthProfile`
- `TunnelRule`
- `SessionTab`
- `WorkspaceLayout`
- `Snippet`
- `ThemeProfile`
- `BackgroundProfile`
- `RecentItem`
- `KnownHostEntry`
- `CommandHistoryItem`

### UI 结构

- 左侧: 主机、分组、最近连接、Snippets、隧道、密钥、设置
- 中间: 终端标签页、分屏、远程命令结果
- 右侧: 当前连接、SFTP、转发、日志、主机信息
- 底部: 状态栏、连接状态、传输进度、延迟、快捷入口
- 全局: 命令面板、搜索、快速连接、主题与背景控制

### 当前工作区界面决策

- 默认进入 Hosts 页面，顶部保留低高度工具栏，左侧使用窄图标导航。
- C 区承载 Hosts 资源列表，支持 List / Card 视图状态、分组/标签/状态过滤入口。
- D 区承载多连接标签页，终端为默认主工作区，SFTP 可作为独立页面或后续终端内分栏。
- E 区为右侧活动/主机信息栏，支持折叠为窄栏以扩大 D 区。
- 全局命令面板从顶部搜索入口打开，覆盖 Hosts、Snippets、Command History、Settings。
- 界面文案走中英文 i18n key，默认语言状态为跟随系统。
- 内置主题状态包含 Professional Dark、Ocean Dark、Forest Dark，后续接入设置页切换。
- 背景图片轮播状态已接入，支持本地/在线图片源、透明度、模糊度和下一张切换。

### 关键实现点

- 终端渲染需要自定义封装，`iced` 不提供现成的完整终端组件
- SSH 与 SFTP 需要共享连接上下文，避免重复认证和重复建连
- 背景图片轮转与模糊/透明叠加要和终端绘制层解耦
- 主机级配置覆盖全局配置，但必须保留可回退路径
- `ssh-agent`、证书、Known Hosts 需要跨平台兼容处理

### 风险点

- 终端组件的输入法、选择、复制粘贴、滚动性能
- 代理、跳板机、端口转发的状态同步
- 凭据和 agent 的平台差异
- 背景图片与终端文字的可读性
- SFTP 大目录与批量传输的性能

## 推荐开发顺序

1. 统一数据模型与存储层
2. SSH 认证、连接和终端主链路
3. 标签页、分屏、最近连接、主机管理
4. SFTP 面板与传输队列
5. 端口转发与隧道管理
6. 主题、背景轮转、视觉配置
7. 本地终端、日志、监控
8. 同步、扩展协议、AI

## 代码组织原则

- 按功能域模块化，不按界面零散拆文件
- 每个模块只负责一个清晰能力边界
- 控制文件数量，避免过度碎片化
- 公共数据模型与业务逻辑优先集中管理
- UI、状态、服务、存储分层明确
- 单文件只放同一职责内的相关代码，避免一个文件塞进多个领域
- 先做大模块，再在模块内部做最小必要拆分
- 共享类型、常量、配置键统一放在稳定位置

## 建议目录

```text
src/
  main.rs
  app.rs
  model.rs
  config.rs
  ui/
    mod.rs
    shell.rs
    theme.rs
    background.rs
  session/
    mod.rs
    ssh.rs
    auth.rs
    history.rs
  terminal/
    mod.rs
    core.rs
    input.rs
    render.rs
  sftp/
    mod.rs
    browser.rs
    transfer.rs
  forward/
    mod.rs
    rules.rs
  storage/
    mod.rs
    hosts.rs
    workspace.rs
  security/
    mod.rs
    keyring.rs
    known_hosts.rs
```

## 拆分规则

- `main.rs` 只做启动
- `app.rs` 只做应用入口和全局状态装配
- `model.rs` 放核心领域模型
- `config.rs` 放配置结构与加载保存
- 每个功能域最多先保留一个 `mod.rs` 加少量实现文件
- 只有当单文件职责明显膨胀时，再继续拆分
- UI 文件按“页面/面板”拆，不按控件细粒度拆
- 逻辑文件按“领域行为”拆，不按工具函数随手拆
- 不为未来可能性预拆分，优先满足当前需求

## 首版建议落点

- 先保留少量核心文件
- 优先保证模块边界清晰
- 代码体量增长后，再按领域扩展子文件
- 避免一开始就生成大量小文件，降低维护成本

## 首批代码骨架

### 第一阶段目标

- 先打通应用启动、配置加载、主窗口、主状态、SSH 连接、终端显示
- 先让核心链路闭环，再逐步补 SFTP、隧道、历史、主题、背景
- 所有后续功能都围绕这一条主链路扩展

### 推荐首版文件

- `src/main.rs`: 程序入口，只负责启动
- `src/app.rs`: 应用状态、消息路由、全局初始化
- `src/model.rs`: Host、Session、Theme、Background 等核心模型
- `src/config.rs`: 全局配置与主机覆盖配置的读写
- `src/ui/mod.rs`: UI 总入口
- `src/ui/shell.rs`: 主布局、侧边栏、标签页、分屏
- `src/ui/theme.rs`: 主题切换与配色
- `src/ui/background.rs`: 背景图片轮转、透明度、模糊度
- `src/session/mod.rs`: 会话总入口
- `src/session/ssh.rs`: SSH 连接与数据流
- `src/session/auth.rs`: 认证方式
- `src/terminal/mod.rs`: 终端总入口
- `src/terminal/core.rs`: 终端状态与缓冲
- `src/terminal/input.rs`: 键盘、鼠标、复制粘贴
- `src/terminal/render.rs`: 终端绘制
- `src/storage/mod.rs`: 存储总入口
- `src/storage/hosts.rs`: 主机、分组、最近连接
- `src/storage/workspace.rs`: 工作区、标签页、布局
- `src/security/mod.rs`: 安全总入口
- `src/security/keyring.rs`: 凭据存储
- `src/security/known_hosts.rs`: Known Hosts

### 首版边界

- UI 只负责展示和交互
- `session` 只负责连接生命周期
- `terminal` 只负责终端状态与渲染
- `storage` 只负责持久化
- `security` 只负责凭据与校验
- `config` 只负责配置结构和落盘

### 第一批实现顺序

1. `model` 与 `config`
2. `storage` 与 `security`
3. `session` 的 SSH 主链路
4. `terminal` 的基础渲染和输入
5. `ui` 的主布局和标签页
6. 背景与主题
7. 再接 SFTP 和端口转发

## 交互流程

1. 启动应用，读取全局配置、主机列表、最近连接和工作区状态
2. 选择主机或搜索主机，加载主机覆盖配置
3. 完成认证校验，建立 SSH 连接
4. 打开终端标签页，进入交互式 shell 或远程命令执行
5. 在同一会话中切换到 SFTP、端口转发、日志、历史和设置面板
6. 断线、失败或重连时，保留会话上下文和最近状态
7. 退出时保存工作区、最近连接、布局、主题和背景状态

## 状态模型

### 应用状态

- 当前窗口布局
- 当前活动标签页
- 全局主题
- 全局背景
- 全局配置
- 加载状态

### 连接状态

- 未连接
- 连接中
- 认证中
- 已连接
- 重连中
- 失败

### 会话状态

- 终端输入输出缓冲
- 当前远程目录
- 标签页类型
- 分屏布局
- 命令历史
- SFTP 浏览位置
- 转发规则状态

### 视觉状态

- 当前主题
- 当前主机主题覆盖
- 背景图片列表
- 当前背景图
- 透明度
- 模糊度
- 轮转间隔

## 首版验收标准

- 可以启动并显示主窗口
- 可以保存和打开主机
- 可以用密码、公钥、`ssh-agent` 建连
- 可以完成 PTY 终端交互
- 可以执行远程命令
- 可以打开多个标签页
- 可以进行基础分屏
- 可以浏览并传输 SFTP 文件
- 可以创建和管理基础端口转发
- 可以保存命令历史
- 可以切换全局主题和主机主题
- 可以启用背景图片轮转、透明度和模糊度
- 可以跨平台运行

## 文件数量目标

- 首版优先控制在少量核心文件
- 先以功能域划分，不追求极细拆分
- 只有模块职责明显增长时才继续拆分
- 优先保证可读性、可维护性和迭代速度

## 开发任务拆分

### 任务 1: 基础工程骨架

- 初始化应用入口
- 统一状态模型
- 统一配置读写
- 统一主窗口布局

### 任务 2: SSH 主链路

- 主机选择
- 认证
- 建连
- 终端收发
- 断线重连

### 任务 3: 终端能力

- PTY 交互
- 输入输出
- 复制粘贴
- 搜索
- 滚动缓冲

### 任务 4: 资产管理

- 主机
- 分组
- 标签
- 最近连接
- 搜索

### 任务 5: SFTP

- 目录浏览
- 上传下载
- 批量传输
- 远程编辑
- 权限操作

### 任务 6: 隧道与转发

- Local
- Remote
- Dynamic
- 规则保存
- 状态显示

### 任务 7: 视觉系统

- 全局主题
- 主机主题覆盖
- 背景轮转
- 透明度
- 模糊度

### 任务 8: 扩展能力

- 本地终端
- 日志
- 监控
- 同步
- 插件
- AI

## 约束清单

- 不做像素级仿制
- 不先拆成大量小文件
- 不把 UI、存储、连接逻辑混在一起
- 不把背景逻辑写死在终端组件里
- 不把主机级配置和全局配置混成一层
- 不依赖单一平台特性作为核心前提
- 不在首版里引入无必要的外部复杂依赖

## 预期输出物

- 可运行桌面程序
- 可保存的需求文档
- 可分模块实现的代码骨架
- 可逐步补全的任务清单

## 核心接口草案

### 应用状态

```rust
struct AppState {
    config: AppConfig,
    workspace: WorkspaceState,
    hosts: Vec<Host>,
    groups: Vec<HostGroup>,
    recent: Vec<RecentItem>,
    sessions: Vec<SessionTab>,
    active_session: Option<SessionId>,
    themes: ThemeState,
    backgrounds: BackgroundState,
    ui: UiState,
}
```

### 主机与连接

```rust
struct Host {
    id: HostId,
    name: String,
    group_id: Option<GroupId>,
    address: String,
    port: u16,
    auth: AuthProfile,
    proxy: Option<ProxyProfile>,
    jump: Vec<JumpProfile>,
    theme_override: Option<ThemeProfile>,
    background_override: Option<BackgroundProfile>,
}

struct ConnectionProfile {
    host_id: HostId,
    ssh: SshProfile,
    terminal: TerminalProfile,
    sftp: SftpProfile,
    forward: Vec<TunnelRule>,
}
```

### 认证

```rust
enum AuthProfile {
    Password { username: String, secret_ref: SecretRef },
    Key { username: String, key_ref: SecretRef, passphrase_ref: Option<SecretRef> },
    Agent { username: String, key_hint: Option<String> },
    Certificate { username: String, key_ref: SecretRef, cert_ref: SecretRef },
}
```

### 会话与终端

```rust
struct SessionTab {
    id: SessionId,
    host_id: HostId,
    kind: SessionKind,
    title: String,
    status: SessionStatus,
    terminal: TerminalState,
    sftp: Option<SftpState>,
    forward: Vec<TunnelRuleState>,
    history: Vec<CommandHistoryItem>,
}
```

### 视觉配置

```rust
struct ThemeProfile {
    name: String,
    palette: ColorPalette,
    font_family: String,
    font_size: f32,
    terminal_style: TerminalStyle,
}

struct BackgroundProfile {
    enabled: bool,
    sources: Vec<ImageSource>,
    rotation_interval_secs: u64,
    opacity: f32,
    blur: f32,
}
```

### 消息流

- `AppMsg`: 应用级消息
- `SessionMsg`: SSH 连接、断线、重连、认证结果
- `TerminalMsg`: 输入、输出、尺寸变化、滚动、选择
- `SftpMsg`: 目录加载、上传、下载、重命名、删除
- `ForwardMsg`: 规则启停、状态变化、错误
- `StorageMsg`: 读取、保存、导入、导出
- `VisualMsg`: 主题切换、背景切换、透明度、模糊度

### 状态流转

- `AppState` 驱动窗口布局和页面切换
- `SessionTab` 驱动单个连接的生命周期
- `TerminalState` 驱动终端渲染和输入输出
- `BackgroundState` 驱动视觉层轮转
- `WorkspaceState` 驱动窗口恢复和标签页恢复

### 约定

- 结构体先定义领域意义，再考虑存储格式
- 消息先按功能域分组，再考虑 UI 事件转发
- 数据模型尽量稳定，避免 UI 细节渗透进核心层
- 连接状态、视觉状态、存储状态彼此解耦

## 模块接口细化

### config

- `load_global_config()`
- `save_global_config()`
- `load_host_config(host_id)`
- `save_host_config(host_id)`
- `merge_host_config(global, host)`

### storage

- `load_hosts()`
- `save_hosts()`
- `load_groups()`
- `save_groups()`
- `load_workspace()`
- `save_workspace()`
- `load_recent()`
- `save_recent()`
- `import_openssh_config()`
- `export_openssh_config()`

### security

- `store_secret()`
- `load_secret()`
- `delete_secret()`
- `list_known_hosts()`
- `add_known_host()`
- `remove_known_host()`
- `verify_host_key()`

### session

- `connect()`
- `disconnect()`
- `reconnect()`
- `authenticate()`
- `open_shell()`
- `exec_command()`
- `open_sftp()`
- `open_forward()`

### terminal

- `feed_output()`
- `handle_input()`
- `resize()`
- `copy_selection()`
- `paste_text()`
- `search()`
- `scroll()`

### sftp

- `read_dir()`
- `upload_file()`
- `download_file()`
- `rename_entry()`
- `remove_entry()`
- `set_permissions()`
- `sync_dir()`

### forward

- `add_rule()`
- `remove_rule()`
- `start_rule()`
- `stop_rule()`
- `query_status()`

### visual

- `set_theme()`
- `set_host_theme_override()`
- `set_background()`
- `start_rotation()`
- `stop_rotation()`
- `update_opacity()`
- `update_blur()`

## 关键数据约束

- `HostId`、`GroupId`、`SessionId` 使用稳定唯一标识
- `SecretRef` 只保存引用，不直接暴露明文
- `ThemeProfile` 和 `BackgroundProfile` 支持全局与主机覆盖
- `SessionTab` 持有会话运行态，`WorkspaceLayout` 持有布局态
- `TunnelRule` 必须可持久化、可启停、可重载
- `CommandHistoryItem` 需要支持主机级和全局级历史

## 持久化建议

- 主机、分组、工作区、历史: `redb` 或 `sqlite`
- 配置与主题: `toml`
- 凭据引用: `keyring`
- 大文件缓存和临时传输: 系统临时目录

## 运行时约束

- SSH、SFTP、转发、终端必须共享同一会话上下文
- UI 事件不得直接操作底层网络细节
- 所有长耗时操作都要走异步任务
- 视觉刷新不能阻塞终端输入输出
- 断线重连不得丢失最近会话上下文

## 推荐里程碑

### MVP

- 主机管理
- 认证
- Known Hosts
- PTY 终端
- 远程命令执行
- 多标签页
- 基础分屏
- 基础 SFTP
- 三类端口转发
- 命令历史
- 本地加密配置

### V1

- 本地终端
- 跳板机
- 代理
- Snippets
- 远程编辑
- 工作区恢复
- 日志
- 主题与字体配置
- 快捷键

### V2

- 批量执行
- 监控面板
- 会话录制
- 同步
- 插件协议
- AI 能力

### V3

- Serial
- Zmodem
- RDP
- VNC
- FTP / S3 / WebDAV / SMB

## 当前状态

需求已整理，最小 Rust + Iced 工程骨架已创建。

- 已创建 `Cargo.toml`
- 已创建首批 `src/` 模块
- 已接入最新可用的核心依赖
- 已补首批领域模型
- 已通过 `cargo check`

## 提交规范

- Commit message 首行必须是简短总结
- Commit message 正文必须包含中文和英文双语说明
- 中文说明放在英文说明之前
