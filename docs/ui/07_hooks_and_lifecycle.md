# 🌐 07. Hook 插件与生命周期协同架构 (Hooks & Lifecycle Architecture)

---

## 📌 概述

`smalux-ssh` 采用多层 Hook 插件与生命周期解耦架构，通过 **应用级全局 Hook (`AppGlobalHook`)** 与 **终端流式 Hook (`TerminalHook`)**，实现 UI 表现层、存储层与网络会话之间的高度解耦与无缝扩展。

---

## 🏛️ 1. 双层 Hook 架构分层

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                        应用级全局 Hook (AppGlobalHook)                     │
│  - 负责三栏生命周期协同 (左侧资产抽屉 ⟷ 中央终端视口 ⟷ 右侧伴生工具栏)    │
│  - 负责双盘文件浏览器 (会话导航 ⟷ 高危操作安全守护 ⟷ 传输管道流式度量)    │
│  - 负责全局路由跳转、系统偏好变更、配置自动增量备份与退出前安全拦截       │
└─────────────────────────────────────┬─────────────────────────────────────┘
                                      │
┌─────────────────────────────────────▼─────────────────────────────────────┐
│                        终端流式 Hook (TerminalHook)                       │
│  - 负责 PTY 进程与 VT100 状态机之间的字节流拦截与管道过滤                  │
│  - 负责高危命令拦截 (Dangerous Command Guard)、历史轨迹记录与吞吐度量     │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## 🧭 2. 应用级全局 Hook (`AppGlobalHook`) 规格

### 2.1 三栏协同设计哲学 (Three-Column Decoupling)
1. **左侧资产栏 (`host_asset_`)**：
   - 关注资产 CRUD、分组树拓扑、搜索过滤与探活；
   - 发起终端连接后立即解绑，后续重命名主机或拖拽分组**绝不干扰已运行的终端**。
2. **中央终端视口 (`host_terminal_`)**：
   - 作为单向焦点真相源（Focus Truth），广播当前聚焦会话快照；
   - 被动接收来自右侧工具箱的动作注入。
3. **右侧伴生工具栏 (`host_right_`)**：
   - 伴生观察者与工具箱，监听中央活动终端的焦点切换，自动热重载详情、代码片段、SFTP 与 AI 面板；
   - 支持向中央终端反向注入指令（Action Injection）。

### 2.2 核心 Hook 回调接口一览
| 域 (Domain) | 回调方法 | 触发时机与典型应用 |
| :--- | :--- | :--- |
| **资产抽屉** | `on_host_asset_created/updated/deleted` | 主机 CRUD 成功后触发，驱动启动器增量更新与自动备份 |
| **分组拓扑** | `on_host_asset_group_created/toggled` | 树形分组折叠/展开与拖拽调序时自动持久化 |
| **终端连接** | `on_host_terminal_opening` | **连接前拦截（责任链）**：校验连接配额或前置跳板代理 |
| **焦点广播** | `on_host_terminal_focus_changed` | **焦点流核心**：Tab 切换或多分屏选窗格时广播活跃终端快照 |
| **工具协同** | `on_host_terminal_action_requested` | 右侧工具栏向中央终端 PTY 注入并执行快捷命令 |
| **全域路由** | `on_navigation_requested / completed` | 统一路由中枢 (`navigate_to`) 跳转与历史入栈 |
| **生命周期** | `on_app_before_exit` | **退出守护拦截**：多连接运行检查、保存未落盘状态 |

---

## ⚡ 3. 终端流式 Hook (`TerminalHook`) 规格

### 3.1 核心管道拦截模型
```text
用户键盘输入 ──► on_input() (高危命令拦截) ──► PTY stdin
                                                 │
                                                 ▼
PTY stdout  ──► on_output_raw() (吞吐指标度量) ──► alacritty_terminal (VT100) ──► 120Hz 光栅化渲染
```

### 3.2 核心特性
- **高危命令拦截 (`DangerousCommandGuardHook`)**：拦截危险命令（如 `rm -rf /`、`mkfs`），阻断输入并弹出安全警告；
- **历史记录追踪 (`HistoryTrackingHook`)**：自动记录会话开始/结束时间戳、命令输入历史与总流量；
- **Panic 隔离保护**：所有 Hook 执行均由 `std::panic::catch_unwind` 包装，任何插件 panic 均不会导致终端进程或 UI 崩溃。

---

## 📂 4. 双盘文件浏览器与 SFTP 传输 Hook 体系 (`file_` 域)

### 4.1 三大核心子领域划分
```text
┌──────────────────────────────────────────────────────────────────────────┐
│                    文件系统全局 Hook 体系 (File Domain Hooks)             │
├────────────────────────────────┬─────────────────────────────────────────┤
│ 1. 会话与路径导航域 (file_tab_) │ • SFTP 会话打开前拦截 (权限校验/跳板机)  │
│                                │ • 左右双栏 Tab 焦点广播与伴生感知       │
│                                │ • 目录深度跳转与历史栈追踪              │
├────────────────────────────────┼─────────────────────────────────────────┤
│ 2. 文件操作与安全守护 (file_op_)│ • 高危删除/覆写前置拦截 (系统根目录保护) │
│                                │ • 文件/目录创建与删除完成记录           │
├────────────────────────────────┼─────────────────────────────────────────┤
│ 3. 传输管道与度量 (file_xfer_) │ • 任务入队配额与黑名单过滤               │
│                                │ • 实时传输进度、流速度量与失败重试      │
│                                │ • 传输完成全局气泡通知与增量刷新        │
└────────────────────────────────┴─────────────────────────────────────────┘
```

### 4.2 具体 Hook 方法契约
| 方法名 | 签名 | 说明与应用 |
| :--- | :--- | :--- |
| **`on_file_tab_opening`** | `fn on_file_tab_opening(&self, host_id: &str, initial_path: &str) -> HookDecision` | **连接前拦截（责任链）**：打开 SFTP 会话前触发，可返回 `HookDecision::Abort` 阻断连接。 |
| **`on_file_tab_opened`** | `fn on_file_tab_opened(&self, session_id: &str, host_id: &str, initial_path: &str)` | SFTP 或本地文件 Tab 成功建立并挂载完毕。 |
| **`on_file_tab_focus_changed`** | `fn on_file_tab_focus_changed(&self, session_id: Option<&str>, is_remote: bool, current_path: &str)` | **焦点广播**：双栏 Tab 切换或选栏时广播当前活动路径（供右侧工具栏伴生感知）。 |
| **`on_file_tab_navigated`** | `fn on_file_tab_navigated(&self, session_id: &str, is_remote: bool, from_path: &str, to_path: &str)` | 路径导航跳转后触发（前进、后退、向上、回车直达）。 |
| **`on_file_tab_closed`** | `fn on_file_tab_closed(&self, session_id: &str)` | 文件会话 Tab 关闭，释放底层资源。 |
| **`on_file_operation_before`** | `fn on_file_operation_before(&self, op_type: &str, is_remote: bool, path: &str) -> HookDecision` | **高危操作前置拦截（责任链）**：在删除/覆写前触发，保护敏感路径。 |
| **`on_file_operation_completed`** | `fn on_file_operation_completed(&self, op_type: &str, is_remote: bool, path: &str, success: bool)` | 文件/目录操作完成（创建、删除、重命名）。 |
| **`on_file_transfer_enqueued`** | `fn on_file_transfer_enqueued(&self, task: &TransferTask) -> HookDecision` | 传输任务入队前校验（配额检查、后缀过滤）。 |
| **`on_file_transfer_started`** | `fn on_file_transfer_started(&self, task_id: &str)` | 传输任务开始执行。 |
| **`on_file_transfer_progress`** | `fn on_file_transfer_progress(&self, task_id: &str, transferred: u64, total: u64, speed_bps: u64)` | 传输进度与速率更新（用于度量监控与底部抽屉统计）。 |
| **`on_file_transfer_completed`** | `fn on_file_transfer_completed(&self, task: &TransferTask)` | 传输任务完成（触发 Toast 成功通知与对侧目录增量刷新）。 |
| **`on_file_transfer_failed`** | `fn on_file_transfer_failed(&self, task: &TransferTask, error_message: &str)` | 传输任务失败（触发 Toast 错误通知与重试标记）。 |

### 4.3 内置安全守护插件：`DangerousFileGuardHook`
- **优先级**：`100`（最高优先级，最先执行安全拦截）；
- **保护路径**：`"/"`, `"/etc"`, `"/bin"`, `"/sbin"`, `"/usr"`, `"/lib"`, `"/boot"`, `"/sys"`, `"/proc"`, `"/dev"`, `"C:\Windows"`, `"C:\Windows\System32"`, `"C:\Program Files"`, `"C:\"`, `"D:\"` 等；
- **行为**：当用户尝试对上述路径发起删除指令时，返回 `HookDecision::Abort` 强力阻断，并通过 UI 层的 `ToastNotification` 弹出警示气泡。
