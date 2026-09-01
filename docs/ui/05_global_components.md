# 🧩 05. 全局通用组件库 (Global Components Reference)

---

## 📌 模块概述

Smalux-SSH 抽象了一套高复用度、轻量且视觉统一的全局基础组件库。所有组件均遵从纯净暗色主题标准（无彩度 Zinc 色阶），支持 Slint 声明式数据绑定与强类型回调。

---

## 目录索引

1. [全局气泡通知系统 (ToastContainer / ToastCard)](#1-全局气泡通知系统-toastcontainer--toastcard)
2. [右键上下文菜单 (ContextMenuContainer / ContextMenuItem)](#2-右键上下文菜单-contextmenucontainer--contextmenuitem)
3. [主机资产选择器 (HostPickerList / HostPickerCard)](#3-主机资产选择器-hostpickerlist--hostpickercard)
4. [通用树状分组选择器 (GroupTreeSelector)](#4-通用树状分组选择器-grouptreeselector)
5. [新建主机分组弹窗 (CreateGroupModal)](#5-新建主机分组弹窗-creategroupmodal)
6. [新建终端会话弹窗 (NewSessionModal)](#6-新建终端会话弹窗-newsessionmodal)
7. [选择文件会话弹窗 (FileHostModal)](#7-选择文件会话弹窗-filehostmodal)
8. [会话详情弹窗 (HistoryDetailModal)](#8-会话详情弹窗-historydetailmodal)
9. [统一消息弹窗 (MessageDialog)](#9-统一消息弹窗-messagedialog)
10. [暗色搜索框 (SearchInput)](#10-暗色搜索框-searchinput)
11. [健康状态小圆点 (StatusDot)](#11-健康状态小圆点-statusdot)
12. [细粒度进度条 (ProgressBar)](#12-细粒度进度条-progressbar)
13. [标准图标按钮 (AppIconButton)](#13-标准图标按钮-appiconbutton)
14. [可折叠抽屉容器 (DrawerContainer)](#14-可折叠抽屉容器-drawercontainer)
15. [全局指令面板 (CommandPalette - Ctrl+K)](#15-全局指令面板-commandpalette---ctrlk)

---

## 1. 全局气泡通知系统 (`ToastContainer` / `ToastCard`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/toast.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/toast.slint)
- **Rust 管理器**：[`crates/smagical-ui/src/notification_service.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/notification_service.rs)

### 特性与规范
- **顶层非阻塞 (`z: 1000`)**：浮于全屏最顶层，不遮挡下方常规按钮点击；
- **4 级语义配色与 SVG 矢量图标**：
  - 🟢 **`Success` (2200ms)**：`check-circle.svg`
  - 🔵 **`Info` (2000ms)**：`info-circle.svg`
  - 🟡 **`Warning` (2800ms)**：`alert-triangle.svg`
  - 🔴 **`Error` (3000ms)**：`x-circle.svg`
- **右上角绝对定位关闭按钮 (`✕`)**：无论文本长度如何，关闭按钮稳固在卡片右上角；
- **自动排队与消隐**：超过 5 条时自动挤出最早的通知。

```rust
// Rust 端使用 AppContext 门面调用
ctx.notify_success("操作成功", "文件已完成传输");
ctx.notify_error("路径不存在", format!("无法访问目标路径: {}", path));
```

---

## 2. 右键上下文菜单 (`ContextMenuContainer` / `ContextMenuItem`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/context-menu.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/context-menu.slint)

### 特性
- 纯暗色背景 + 柔和投影 + 细边框 (`z: 800`)；
- 支持危险操作红色高亮 (`is-danger: true`)；
- 支持置灰禁用 (`is-disabled: true`)；
- 支持水平分割线 (`is-separator: true`)；
- 内置屏幕边缘自适应防溢出翻转算法。

---

## 3. 主机资产选择器 (`HostPickerList` / `HostPickerCard`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/host-picker-list.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/host-picker-list.slint)

### 特性
- 统一 44px 高度规范卡片；
- 左侧居中服务器图标，中间为主机名与 IP:Port，右侧为在线状态指示点与所属分组标签；
- 支持自定义悬停强调色（`hover-accent-color`，如终端模式的品牌蓝 vs 文件模式的 SFTP 翡翠绿）；
- 已在 `NewSessionModal`（新建终端）与 `FileHostModal`（新建文件会话）中 100% 共享复用。

---

## 4. 通用树状分组选择器 (`GroupTreeSelector`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/group-tree-selector.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/group-tree-selector.slint)

### 特性
- 左侧独立 `18px` 折叠三角热区，支持点击折叠/展开；
- 整行双击快捷折叠/展开子节点；
- 选中行具备强调色圆点与高亮外框；
- 内置 `Flickable` 平滑滚动与自隐藏 `3px` 悬浮滚动条。

---

## 5. 新建主机分组弹窗 (`CreateGroupModal`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/create-group-modal.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/create-group-modal.slint)

### 特性
- 居中 `460px x 420px` 精致深色卡片，内容 100% 满宽排布；
- 上半部内嵌 `200px` 高度树形所属上级选择器；
- 下半部输入框采用纯暗色设计，左侧固定文件夹图标，右侧绝对锚定 `✕` 一键清空按钮；
- 支持 `Enter` 回车快速提交创建。

---

## 6. 新建终端会话弹窗 (`NewSessionModal`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/new-session-modal.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/new-session-modal.slint)

### 特性
- 上半部分呈现本地探测到的 Shell 环境（PowerShell 7, WSL, Git Bash, CMD 等）；
- 下半部分接入 `HostPickerList` 展示已保存的远程主机资产；
- 支持实时关键字搜索与快捷回车创建会话。

---

## 7. 选择文件会话弹窗 (`FileHostModal`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/file-host-modal.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/file-host-modal.slint)

### 特性
- 专用于双盘文件管理页面的快速会话连接器；
- 上半部分为单一条目「💻 本地文件系统 (Local Filesystem)」；
- 下半部分接入 `HostPickerList`（SFTP 翡翠绿高亮）。

---

## 8. 会话详情弹窗 (`HistoryDetailModal`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/history-detail-modal.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/history-detail-modal.slint)

### 特性
- 居中展示已归档会话的元数据信息；
- 展示会话持续时长、发起时间、退出状态码、累计输入命令与网络总吞吐指标。

---

## 9. 统一消息弹窗 (`MessageDialog`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/message-dialog.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/message-dialog.slint)

### 特性
- 用于阻断性或关键二次确认操作（如删除分组、清空历史）；
- 支持 `error`、`warning`、`info` 三类语义模态提示。

---

## 10. 暗色搜索框 (`SearchInput`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/search-input.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/search-input.slint)

### 特性
- 统一暗色输入框背景与高对比度边框；
- 左侧内嵌搜索放大镜图标；
- 右侧自带动态 `✕` 清空按钮（文本不为空时显现）。

---

## 11. 健康状态小圆点 (`StatusDot`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/status-dot.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/status-dot.slint)

### 特性
- 标准 4 态渲染：`online` (绿), `warning` (黄), `error` (红), `offline` (灰)；
- 支持自定义 `dot-size`（默认 6px）。

---

## 12. 细粒度进度条 (`ProgressBar`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/progress-bar.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/progress-bar.slint)

### 特性
- 紧凑 4px ~ 6px 高度槽位；
- 支持 0.0 ~ 1.0 平滑进度条动画，支持自定义进度槽颜色与背景色。

---

## 13. 标准图标按钮 (`AppIconButton`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/base/app-icon-button.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/base/app-icon-button.slint)

### 特性
- 20px ~ 24px 精致正方形热区；
- 鼠标悬停半透明高亮底色与图标高亮滤镜。

---

## 14. 可折叠抽屉容器 (`DrawerContainer`)

- **Slint 组件**：[`crates/smagical-ui/ui/components/drawer-container.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/drawer-container.slint)

### 特性
- 统一左侧所有二级抽屉（主机、历史、设置等）的标题栏、折叠按钮与分割线排版。

---

## 15. 全局指令面板 (`CommandPalette` - Ctrl+K)

- **Slint 组件**：[`crates/smagical-ui/ui/components/command-palette.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/command-palette.slint)

### 特性
- 居中悬浮模糊搜索面板（对标 VS Code `Ctrl+P` / `Ctrl+Shift+P`）；
- 支持快捷切换主机、调整主题模式、分屏与执行运维命令。
