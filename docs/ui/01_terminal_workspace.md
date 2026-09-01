# 💻 01. 终端多窗格与会话工作区 (Terminal Workspace)

---

## 📌 模块概述

终端会话工作区是 Smalux-SSH 的核心主交互区，支持 **多会话 Tab 标签调度**、**无限嵌套多窗格分屏 (Infinite Multi-Split Panes)** 与 **120Hz 高性能光栅化渲染引擎**。

---

## 🧩 UI 组件与文件结构

- **视图入口**：[`crates/smagical-ui/ui/views/center_terminal/terminal_viewport.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/center_terminal/terminal_viewport.slint)
- **Tab 标签栏**：[`crates/smagical-ui/ui/views/center_terminal/tab_bar.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/center_terminal/tab_bar.slint)
- **窗格分割条**：[`crates/smagical-ui/ui/components/splitter.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/splitter.slint)
- **渲染核心**：[`crates/smagical-ui/src/terminal/renderer.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/terminal/renderer.rs)
- **分屏树布局**：[`crates/smagical-ui/src/terminal/split_tree.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/terminal/split_tree.rs)
- **按键编码器**：[`crates/smagical-ui/src/terminal/key_encoder.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/terminal/key_encoder.rs)
- **路由回调**：[`crates/smagical-ui/src/handlers/session_handlers.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/handlers/session_handlers.rs)

---

## 📐 核心数据契约 (Data Contracts)

### 1. Tab 标签项 (`TabData`)
```slint
export struct TabData {
    id: string,             // 会话唯一 ID
    title: string,          // 显示标题 (如 "Ubuntu-Prod #1")
    is-active: bool,        // 是否激活
    status: string,         // "online" | "connecting" | "offline"
    host-id: string,        // 关联主机 ID
}
```

### 2. 窗格数据模型 (`TerminalPaneData`)
```slint
export struct TerminalPaneData {
    id: string,             // 窗格 ID (如 "pane-1")
    pixel-x: length,        // 绝对像素 X 坐标
    pixel-y: length,        // 绝对像素 Y 坐标
    pixel-width: length,    // 绝对像素宽度
    pixel-height: length,   // 绝对像素高度
    is-focused: bool,       // 当前焦点状态
    is-zoomed: bool,        // 是否单窗格最大化展示
    frame: image,           // 120Hz 渲染出的光栅化位图 Image 句柄
    tabs: [TabData],        // 该窗格内部包含的 Tab 列表
    active-tab-id: string,  // 该窗格当前激活的 Tab ID
}
```

### 3. 分割条模型 (`TerminalSplitterData`)
```slint
export struct TerminalSplitterData {
    id: string,             // 分割条 ID
    is-vertical: bool,      // true: 垂直分割线, false: 水平分割线
    pixel-x: length,        // X 坐标
    pixel-y: length,        // Y 坐标
    pixel-length: length,   // 分割线长度
    splitter-width: length, // 响应热区厚度 (默认 4px)
}
```

---

## ⚡ 核心交互机制

1. **PTY 异步流与 120Hz (8ms) 定时渲染**：
   - 后台 Tokio 任务持续泵送 PTY 输出字节流进入 `alacritty_terminal` 状态机；
   - Slint 主循环定时器每 8ms 触发一次增量/全量光栅化；
   - 直接向 `TerminalPaneData.frame` 写入 `slint::Image::from_rgba8` 缓冲区，零卡顿。
2. **无限分屏 (Split Tree)**：
   - 支持向右水平分屏 (`SplitHorizontal`) 与向下垂直分屏 (`SplitVertical`)；
   - 拖拽分割条时，通过 `split_tree.resize_split(ratio)` 实时重算所有子窗格像素几何尺寸。
3. **文本选择与双向剪贴板**：
   - 鼠标左键拖拽选中文本产生蓝色高亮选区；
   - 支持自动复制选中文本到系统剪贴板（通过 `arboard`）；
   - 鼠标右键或 `Shift+Insert` 直接粘贴剪贴板内容并写入 PTY stdin。
4. **Tab 页签拖拽重排与跨分屏移动合并**：
   - **同窗格拖拽重排**：在同一个终端分屏内拖拽 Tab 页签，松开后平滑更新 `tabs` 顺序并保持当前激活焦点；
   - **跨分屏自由拖拽**：支持将 Tab 从窗格 A 拖拽落入窗格 B 的 Tab 栏，Tab 将从窗格 A 移出并插入窗格 B；
   - **末位 Tab 迁出自动合并关闭原窗格**：若迁出的 Tab 是窗格 A 中的唯一/最后一个会话，迁出后窗格 A 会自动关闭，二叉树拓扑调用 `SplitTree::close_pane` 回缩分支并重新计算剩余窗格几何尺寸；当剩余窗格数归并为 1 时，系统自动回缩并退出分屏模式；
   - **极简浮动虚影与非法落点置灰**：拖拽时展现半透明卡片虚影（仅包含终端图标、状态指示灯与会话标题），当拖出视口或移入无效区域时，虚影自动呈现置灰状态（红色警示边框、低不透明度）并在释放时自动取消拖拽并恢复原位。
