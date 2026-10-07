# Slint UI 微前端与组件体系 (`crates/ui`)

`crates/ui` 是 **smalux-ssh** 桌面客户端的现代化界面表示层，基于 [Slint UI](https://slint.dev/) 声明式标记语言与 Rust 构建。它采用微内核加插件解耦（Microkernel & Modular Plugins）架构，将庞大的单一 UI 拆分为基础设计系统、轻量壳体内核与 8 大独立业务插件，彻底规避了单文件过大导致的 LLVM 编译内存暴涨。

---

## 📁 目录结构与架构分层

```text
crates/ui/
├── README.md               # 本文档：UI 架构全景与使用指南
├── common/                 # 跨插件共享设计系统 (Design System & Base Components)
│   ├── Cargo.toml          # 纯静态资源与样式包 (仅依赖 slint)
│   ├── README.md           # 设计系统、令牌规范与全局桥接接口文档
│   └── ui/                 # 基础控件、复合组件、主题令牌与全局单例 Bridge
│
├── kernel/                 # 微内核底座 (Shell Microkernel)
│   ├── Cargo.toml          # 包含全局唯一 build.rs (slint_build::compile)
│   ├── README.md           # 微内核生命周期、多窗格视口与窗口装饰器文档
│   ├── build.rs            # Slint 统一编译中心 (扫描聚合所有插件 UI)
│   ├── src/lib.rs          # AppWindow 导出与全局 Slint 类型绑定
│   └── ui/                 # 顶层窗口外壳、无边框标题栏、活动栏、弹窗遮罩层
│
└── plugins/                # 8 大页面级独立业务插件 (与左侧活动栏 1:1 对齐)
    ├── README.md           # 插件系统规范与动态插拔指南
    ├── hosts/              # [页面 1: 主机资产与伴生面板] (AI/监控/Tmux)
    ├── files/              # [页面 2: 双盘文件管理器与 SFTP] (传输队列抽屉)
    ├── snippets/           # [页面 3: 代码片段库与参数化执行] (快速命令抽屉)
    ├── tunnels/            # [页面 4: 网络端口转发与跳板机拓扑] (隧道控制抽屉)
    ├── credentials/        # [页面 5: 凭据保险箱与密钥生成]
    ├── history/            # [页面 6: 连接审计历史与快照重现]
    ├── settings/           # [页面 7: 偏好设置外观与工坊]
    └── debug/              # [页面 8: 开发者调试控制台与状态探针]
```

---

## 🏛️ 核心架构思想

1. **单点编译，多点开发 (Single Build Hub)**：
   Slint 声明式组件通过 `crates/ui/kernel/build.rs` 统一进行 AOT 编译，通过 `include_modules!()` 生成 Rust 强类型结构，其他插件 crate 专注于组件定义，避免多 crate 重复编译 Slint 导致的二进制膨胀。
2. **主题设计系统驱动 (Theme Token System)**：
   所有颜色、间距、圆角与字体均由 `crates/ui/common/ui/theme.slint` 中的 `AppTheme` 统一驱动，支持 15+ 款内置预设与动态浅深色切换，严禁在业务插件中硬编码像素颜色。
3. **Rust - Slint 纯契约解耦 (Bridge Pattern)**：
   业务逻辑严禁写入 Slint 回调，统一在 `crates/ui/common/ui/bridges/` 中定义 `global XXXBridge`，由 Rust 宿主层 (`smagical-ui`) 挂载具体异步事件管道与存储门面。

---

## 🔗 子模块导航

- 🎨 **[设计系统与通用组件 (`crates/ui/common`)](file:///F:/code/rust/smalux-ssh/crates/ui/common/README.md)**：包含 `AppTheme`、原子控件、抽屉脚手架、弹窗基础层与 13 大 Bridge 单例。
- 🖥️ **[微内核底座 (`crates/ui/kernel`)](file:///F:/code/rust/smalux-ssh/crates/ui/kernel/README.md)**：包含主窗口 `AppWindow`、全向无边框拉伸检测、活动栏容器、全局遮罩。
- 🧩 **[8 大业务插件矩阵 (`crates/ui/plugins`)](file:///F:/code/rust/smalux-ssh/crates/ui/plugins/README.md)**：包含主机、文件、片段、隧道、凭据、历史、设置、调试各插件详细实现。
