# smagical-ui-kernel: 桌面微内核底座与唯一 Slint 构建枢纽

> **模块定位**：Smalux 桌面客户端的核心视口与微内核宿主。它负责承载主窗口生命周期 (`AppWindow`)、沉浸式无边框窗口交互与八向缩放手柄、顶层布局路由拓扑（活动栏、多终端 TabBar、侧边抽屉槽、中心主视口、伴生工具栏、底部状态栏），并作为**全工作区唯一的 Slint DSL 宏编译入口 (`build.rs`)**。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **全工作区单一构建总装枢纽 (`build.rs`)**：
   - 遵从极速编译规范，全工程仅在 `crates/ui/kernel` 维护单一 `build.rs`；
   - 通过 `.with_library_paths()` 统一挂载跨插件共享库 `@common` 与 8 大页面级独立插件（`@plugin-hosts`, `@plugin-files`, `@plugin-credentials`, `@plugin-snippets`, `@plugin-tunnels`, `@plugin-history`, `@plugin-settings`, `@plugin-debug`）；
   - 集成国际化多语言 `.po` 字典打包与动态上下文解析。
2. **沉浸式无边框窗口底座 (`ui/kernel.slint`)**：
   - 原生隐藏操作系统标题栏 (`no-frame: true`)，采用自绘制窗口控件与平滑圆角边框；
   - 提供 8 个方位（North, South, East, West, NE, NW, SE, SW）的原生拖拽尺寸调整手柄；
   - 提供全局壁纸双缓冲渲染层（平滑淡入淡出 Crossfade，切图 0 闪烁）。
3. **顶层视口拓扑路由器**：
   - 协调 8 大主视图（`terminal`, `files`, `credentials`, `snippets`, `tunnels`, `history`, `settings`, `debug`）与左侧可伸缩抽屉、右侧伴生工具抽屉之间的自适应弹性伸缩。

---

## 二、 核心 Rust 类型与窗口句柄 (Rust Types & Methods)

通过 `smagical_ui_kernel::AppWindow` 暴露顶层窗口接口：

### 1. 窗口创建与显示
```rust
pub struct AppWindow { /* Slint 内部句柄 */ }

impl AppWindow {
    /// 实例化主窗口及其绑定的 GPU/Winit 渲染表面
    pub fn new() -> Result<Self, slint::PlatformError>;

    /// 展示窗口
    pub fn show(&self) -> Result<(), slint::PlatformError>;

    /// 隐藏窗口 (常用于最小化到系统托盘)
    pub fn hide(&self) -> Result<(), slint::PlatformError>;

    /// 获取关联的强类型全局单例 Bridge
    pub fn global<T: slint::Global<'static, Self>>(&self) -> T;
}
```

### 2. 核心原生窗口控制回调 (Window Callbacks)

在 `smagical-ui/src/lib.rs` 中直接挂载的平台回调：

| 回调方法 | 触发时机 | 参数 | 行为说明 |
| :--- | :--- | :--- | :--- |
| `on_minimize_window` | 用户点击顶栏最小化按钮 | 无 | 调用平台层最小化窗口至任务栏或托盘 |
| `on_maximize_window` | 用户点击顶栏最大化/还原 | 无 | 切换窗口全屏最大化或恢复原比例 |
| `on_close_window` | 用户点击顶栏关闭按钮 | 无 | 检查是否有活跃 SSH 会话，若有弹出确认弹窗，无则优雅退出 |
| `on_force_close_window` | 确认弹窗点击强行退出 | 无 | 强制终止所有会话进程并退出应用进程 |
| `on_start_window_drag` | 鼠标在标题空白区域按下拖动 | 无 | 发起操作系统级别的窗口平移移动 |
| `on_start_window_resize` | 鼠标按住窗口边缘或四角拖拽 | `edge: SharedString` | 触发原生 8 方向边缘尺寸缩放（如 `"north"`, `"south-east"`） |

---

## 三、 顶层布局结构图 (Top-Level Layout)

```text
AppWindow (Window)
├── 顶层容器 (Rectangle, 圆角 8px, 边框 1px)
│   ├── 全局背景壁纸层 (Image, 双缓冲淡入淡出)
│   └── VerticalLayout (主垂直轴)
│       ├── TabBar (顶层全宽控制栏 + 终端会话 Tab 栏 + 窗口控制)
│       ├── HorizontalLayout (中间五栏主工作区)
│       │   ├── LeftActivityBar (最左侧 48px 图标栏)
│       │   ├── Left Drawer (左侧二级可伸缩抽屉: Hosts/Files/Tunnels 等)
│       │   ├── Left Splitter (左侧调宽分割条)
│       │   ├── Central View Area (核心主视图: 终端视口 / 全屏独立大页面)
│       │   ├── Right Splitter (右侧调宽分割条，仅终端展开时可见)
│       │   ├── Right Drawer (右侧伴生工具抽屉: Monitor/AI/Tmux/SFTP)
│       │   └── RightToolBar (最右侧 48px 工具图标栏)
│       └── StatusBar (底层全宽状态栏 + 调试台/主题快捷入口)
├── 全局模态弹窗集 (CreateHostModal, CreateGroupModal, DebugModal, ...)
└── ToastContainer (z: 1000 顶层气泡通知)
```

---

## 四、 编译与构建说明 (Build Configuration)

在 `Cargo.toml` 中，本包由以下开发配置严密保护以避免 LLVM OOM：
```toml
[profile.dev.package.smagical-ui-kernel]
debug = 0           # 禁用庞大调试符号
opt-level = 0       # 启用 FastISel 直出机器码
codegen-units = 16  # 16 单元分片，单单元仅处理约 3,700 个函数
```
可在单包模式下独立进行快速检查：
```bash
cargo check -p smagical-ui-kernel
```
