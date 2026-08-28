# smagicalssh (Smalux SSH)

**smagicalssh** 是一款基于 Rust 与 [Slint UI](https://slint.dev/) 构建的高性能、现代化跨平台桌面 SSH 运维工作台。

---

## 🌟 核心特性

- 🚀 **极速原生体验**：采用 Rust 语言开发，具备低内存占用与毫秒级启动响应；
- 🖥️ **无边框现代化视口**：沉浸式深色无边框设计，支持左右双侧可折叠抽屉与灵活拖拽调整；
- 🌲 **树形层级资产管理**：支持无限层级的主机与分组管理，支持级联折叠/展开、超宽内容横向平滑滚动与实时模糊搜索；
- 📂 **独立高复用组件库**：抽离 `GroupTreeSelector`（树形选择器）、`CreateGroupModal`（新建分组弹窗）、`CommandPalette`（全局指令面板）等现代化组件；
- 🎨 **动态主题与配色预设**：内置 15+ 套经典主题（Darcula, Catppuccin, Monokai, One Dark, Nord, GitHub 等），支持深色/浅色一键平滑无缝热切换；
- 🌐 **多语言国际化 (i18n)**：全界面文案采用 Slint `@tr(...)` 与 gettext `.po` 体系管理；
- ⌨️ **极客生产力**：集成 `Ctrl+K` 全局快速启动面板、多终端按键广播、快捷指令片段发送及系统资源实时监控。

---

## 🏗️ 架构与模块分层

仓库采用 Rust Cargo Workspace 多 crate 架构：

```text
smalux-ssh/
├── crates/
│   ├── smagical-core/          # 核心领域模型与业务服务层 (无 UI 依赖)
│   │   ├── src/
│   │   │   ├── domain/         # 主机、分组、凭据、会话领域实体
│   │   │   ├── state/          # 核心状态机与连接管理
│   │   │   └── theme/          # 主题元数据解析与配色令牌
│   │   └── Cargo.toml
│   │
│   └── smagical-ui/            # 桌面 UI 展示与交互层 (基于 Slint)
│       ├── src/
│       │   ├── lib.rs          # UI 事件总线、状态联动与组件装配
│       │   ├── main.rs         # 客户端二进制启动入口
│       │   └── theme/          # 运行时主题动态注入与样式绑定
│       ├── ui/
│       │   ├── main.slint      # 主窗口顶层装配 (AppWindow)
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
cargo run -p smagical-ui
```

### 编译检查与单元测试

```bash
cargo check --workspace
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
