# smagicalssh (Smalux SSH)

**smagicalssh** 是一款基于 Rust 与 [Slint UI](https://slint.dev/) 构建的高性能、现代化、跨平台桌面 SSH 与终端运维工作台。

---

## 🌟 核心特性

- 🚀 **极速原生体验**：采用纯 Rust 语言打造，内存占用低至数十兆，毫秒级冷启动与高帧率流畅动画渲染；
- 🖥️ **无边框现代化视口**：沉浸式深色无边框窗口设计，左右双侧可折叠抽屉、独立视口宽度计算与自适应弹性伸缩；
- 🌲 **无限层级资产管理**：支持多层级主机与文件夹分组管理，支持级联折叠/展开、超宽节点横向平滑拖拽滚动与实时模糊搜索；
- 🗄️ **解耦存储抽象层**：核心层定义 `AppStorage` / `HostRepository` / `GroupRepository` 标准 CRUD Trait 体系，内置内存种子引擎 `MockStorage`，便于无缝接入 SQLite、JSON 文件或云端存储；
- 🐚 **跨平台本地 Shell 动态探测**：启动时自动扫描并缓存当前系统的 PowerShell 7、Windows PowerShell、WSL、Git Bash、CMD、Bash、Zsh、Fish、Nushell 等终端环境，支持一键新建本地会话；
- 🛠️ **开发者调试工作台 (Debug Workbench)**：内置 `smagical-debug`  crate，提供全系统 Tracing 实时滚动日志抽屉、海量资产批量生成引擎、场景预设（K8s 集群/微服务/大规模压测）一键注入与快速状态模拟；
- 📂 **高复用独立组件库**：抽离 `GroupTreeSelector`（树形选择器）、`CreateGroupModal`（新建分组弹窗）、`CommandPalette`（全局指令面板）等组件；
- 🎨 **专业动态主题系统**：内置 15+ 套经典配色预设（Darcula, Catppuccin, Monokai, Nord, One Dark, Dracula, GitHub 等），支持深色/浅色一键平滑无缝热切换与 Windows Terminal 配色导入；
- 🌐 **多语言国际化 (i18n)**：全界面文案采用 Slint `@tr(...)` 与 gettext `.po` 体系管理；
- ⌨️ **极客生产力**：集成 `Ctrl+K` 全局快速启动面板、多终端按键广播、快捷指令片段发送及系统资源实时监控。

---

## 🏗️ 架构与 Workspace 模块分层

仓库采用 Rust Cargo Workspace 多 crate 分层解耦架构：

```text
smalux-ssh/
├── crates/
│   ├── smagical-core/          # 核心领域模型与业务逻辑层 (纯 Rust，无 UI 依赖)
│   │   ├── src/
│   │   │   ├── domain/         # 主机 (HostRecord, HostStatus)、分组 (GroupRecord) 领域实体
│   │   │   ├── state/          # 核心应用状态 (CoreState)，持有 Arc<dyn AppStorage>
│   │   │   ├── storage/        # 存储抽象 Trait (HostRepository, GroupRepository, AppStorage) & MockStorage
│   │   │   └── theme/          # 主题元数据模型、TOML 解析器、校验器与仓库实现
│   │   └── Cargo.toml
│   │
│   ├── smagical-debug/         # 开发者调试与测试支撑 crate
│   │   ├── src/
│   │   │   ├── tracing_layer.rs# 全局 Tracing 日志收集、内存环形缓冲与按天滚动持久化
│   │   │   ├── batch.rs        # 批量资产生成器 (BatchGenerateConfig)
│   │   │   ├── presets.rs      # 场景预设引擎 (Minimal, K8s, Microservices, Stress 100+)
│   │   │   ├── inspector.rs    # 树形节点自适应宽度测量与调试工具
│   │   │   └── models.rs       # 调试通用轻量节点模型 (DebugRawNode)
│   │   └── Cargo.toml
│   │
│   └── smagical-ui/            # 桌面 UI 展示与交互装配层 (基于 Slint UI)
│       ├── src/
│       │   ├── lib.rs          # 桌面应用入口、事件总线与 Slint 回调路由
│       │   ├── main.rs         # 客户端可执行二进制启动入口
│       │   ├── tree_model.rs   # 树形视图纯函数操作层 (RawTreeNode, 排序, 拖拽, 搜索过滤)
│       │   ├── session.rs      # 终端会话管理与 Slint UI 状态同步
│       │   ├── debug_ui.rs     # Tracing 日志面板数据桥接
│       │   ├── local_shells.rs # 跨平台本地 Shell 环境探测与缓存引擎
│       │   └── theme/          # 运行时主题动态注入与样式令牌绑定
│       ├── ui/
│       │   ├── main.slint      # 顶层主窗口组件 (AppWindow)
│       │   ├── components/     # 通用原子 UI 组件库
│       │   ├── themes/         # 主题样式规范与 TOML 预设配置
│       │   └── views/          # 活动栏、抽屉、终端视口、状态栏等业务视图
│       ├── extract-translations.ps1 # i18n 多语言提取脚本
│       ├── messages.po         # 国际化翻译文件
│       └── Cargo.toml
└── README.md
```

---

## 🚀 快速开始

### 前置要求

- [Rust 工具链](https://rustup.rs/) (1.75+，推荐 stable)
- C++ 编译器（MSVC 或 GCC/Clang，用于 Slint 原生窗口后端编译）

### 运行应用

```bash
# 启动桌面 UI 客户端
cargo run -p smagical-ui
```

### 编译检查与单元测试

```bash
# 全 Workspace 严格静态检查 (0 警告)
cargo clippy --workspace --all-targets -- -D warnings

# 全 Workspace 自动化单元测试 (42 项测试全部通过)
cargo test --workspace
```

---

## 🌐 国际化 (i18n)

UI 字符串统一使用 Slint `@tr(...)` 宏包裹，支持通过 `slint-tr-extractor` 工具一键提取：

- **文案文件**：[`crates/smagical-ui/messages.po`](crates/smagical-ui/messages.po)
- **提取脚本**：
  ```powershell
  & 'crates/smagical-ui/extract-translations.ps1'
  ```

---

## 📄 开源许可证

本项目采用 MIT / Apache-2.0 双重开源许可证。
