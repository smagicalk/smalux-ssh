# 🌐 07. 泛型事件分发与生命周期协同架构 (Event Dispatcher & Lifecycle Architecture)

---

## 📌 概述

`smalux-ssh` 采用基于强类型泛型事件分发器（`EventDispatcher`）与集中式事件管理器（`EventManager`）的现代化事件解耦架构。系统通过 **全局共享总线 (`global`)**、**页面独立分发器 (`pages`)** 与 **组件级分发器 (`components`)**，实现 UI 表现层、存储持久层与异步后台任务之间的高度解耦、安全并发、无死锁与无缝扩展。

---

## 🏛️ 1. 事件分发体系架构

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                     集中式事件管理器 (EventManager)                         │
│  - 负责全局共享总线 (Global Bus) 与生命周期管理                            │
│  - 负责三栏生命周期协同 (左侧资产抽屉 ⟷ 中央终端视口 ⟷ 右侧伴生工具栏)    │
│  - 负责双盘文件浏览器 (会话导航 ⟷ 前置安全审查守卫 ⟷ 传输管道流式度量)    │
│  - 负责全域路由跳转、系统偏好变更、配置审计与退出前安全拦截               │
└─────────────────────────────────────┬─────────────────────────────────────┘
                                      │
┌─────────────────────────────────────▼─────────────────────────────────────┐
│                     泛型事件分发器 (EventDispatcher)                       │
│  - 基于 TypeId 哈希索引直达 (O(1) 查找时间复杂度)                          │
│  - 极小化读写锁临界区 (无锁克隆 listeners 列表后再触发回调，彻底防死锁)    │
│  - 监听者 Panic 沙箱隔离 (std::panic::catch_unwind 保护 UI 线程永不崩溃)   │
│  - RAII 自动生命周期守卫 (ListenerGuard 析构自动反注册，支持 .detach())    │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## 🧭 2. 核心领域强类型事件 (Domain Events) 一览

### 2.1 凭据与安全审计域
| 事件类型 | 载荷说明 | 业务应用与审计说明 |
| :--- | :--- | :--- |
| **`CredentialSavedEvent`** | `cred_id, name, cred_type, algorithm, fingerprint, is_new` | 凭据新建/更新广播，驱动凭据列表增量更新与安全日志记录 |
| **`CredentialDeletedEvent`** | `cred_id` | 凭据从安全保管库物理清除广播 |
| **`CredentialSecretCopiedEvent`** | `cred_id, copy_type, is_sensitive` | 机密提取审计事件（复制密码/私钥自动标记为高敏安全审计） |
| **`KeyGeneratedEvent`** | `algorithm, fingerprint` | 密钥生成成功事件 |
| **`PasswordGeneratedEvent`** | 无 (脱敏标记) | 密码生成成功事件（严格物理脱敏，杜绝明文密码泄露） |

### 2.2 主机资产与分组拓扑域
| 事件类型 | 载荷说明 | 业务应用与审计说明 |
| :--- | :--- | :--- |
| **`HostAssetChangedEvent`** | `action, host_id` | 主机 CRUD 变动，驱动启动器异步预热更新 |
| **`HostGroupToggledEvent`** | `group_id, is_expanded` | 树形分组折叠/展开持久化 |
| **`HostTreeReorderedEvent`** | `from_index, to_index` | 资产树拖拽调序事件 |
| **`HostSearchFilteredEvent`** | `query` | 资产搜索过滤查询事件 |

### 2.3 终端会话与分屏视口域
| 事件类型 | 载荷说明 | 业务应用与审计说明 |
| :--- | :--- | :--- |
| **`TerminalSessionEvent`** | `session_id, host_id, action` | 终端会话开启/关闭事件 |
| **`TerminalSplitChangedEvent`** | `group_count, active_pane_id, is_split` | 分屏切分、合并、Tab 跨窗格拖拽迁移 |
| **`TerminalFocusChangedEvent`** | `session_id, host_id` | **焦点流核心**：聚焦变化时广播，驱动右侧伴生工具栏热重载 |
| **`TerminalActionRequestedEvent`** | `session_id, action` | 右侧工具栏向中央终端 PTY 注入并执行快捷命令 |

### 2.4 双盘文件浏览器与 SFTP 传输域
| 事件类型 | 载荷说明 | 业务应用与审计说明 |
| :--- | :--- | :--- |
| **`FileTabOpeningEvent`** | `host_id, path` (支持 `.abort(reason)`) | **前置安全审查守卫**：打开会话前校验连接权限 |
| **`FileTabOpenedEvent`** | `tab_id, host_id, path` | 文件 Tab 成功创建并完成初始扫描 |
| **`FileTabFocusChangedEvent`** | `tab_id, is_remote, current_path` | 双栏文件 Tab 焦点切换广播 |
| **`FileTabNavigatedEvent`** | `tab_id, is_remote, old_path, new_path` | 路径跳转直达与历史前进/后退 |
| **`FileTabClosedEvent`** | `tab_id` | 文件会话 Tab 关闭，释放资源 |
| **`FileOperationBeforeEvent`** | `action, is_remote, path` (支持 `.abort(reason)`) | **高危操作前置审查守卫**：内置规则拦截删除系统根目录 |
| **`FileOperationCompletedEvent`** | `action, is_remote, path, success` | 文件/文件夹创建或删除完成 |
| **`FileTransferStartedEvent`** | `task_id` | 文件传输任务启动 |

### 2.5 全局路由与应用生命周期域
| 事件类型 | 载荷说明 | 业务应用与审计说明 |
| :--- | :--- | :--- |
| **`AppBootEvent`** | 无 | 应用引导启动事件，驱动后台异步探测本地 Shell 环境 |
| **`AppReadyEvent`** | 无 | 应用首帧渲染就绪，触发启动器资产后台预热 |
| **`AppBeforeExitEvent`** | `active_session_count` (支持 `.abort(reason)`) | **退出前安全守护**：检查未落盘数据与活跃连接 |
| **`AppExitEvent`** | `exit_code` | 应用退出终态事件 |
| **`ThemeChangedEvent` / `ThemeModeToggledEvent`** | `theme_id, is_dark` | 配色主题与深浅色模式切换 |
| **`WindowStateChangedEvent`** | `state` ("minimized", "maximized", "restored") | 窗口状态切换 |
| **`ConfigChangedEvent`** | `key, old_val, new_val, source` | 配置动态变更通知与自动增量备份 |

### 2.6 代码片段与脚本资产域 (Code Snippets & Execution)
| 事件类型 | 载荷说明 | 业务应用与审计说明 |
| :--- | :--- | :--- |
| **`SnippetSavedEvent`** | `snippet_id, title, parent_group_id, is_new` | 代码片段新建/编辑保存广播，触发日志审计与视图同步 |
| **`SnippetDeletedEvent`** | `snippet_id` | 代码片段从存储库删除事件 |
| **`SnippetGroupSavedEvent`** | `group_id, name, parent_id, is_new` | 代码片段文件夹新建/编辑保存事件 |
| **`SnippetGroupDeletedEvent`** | `group_id` | 代码片段文件夹物理删除与子项归集事件 |
| **`SnippetExecutedEvent`** | `snippet_id, session_id, auto_execute` | 代码片段向终端注入/执行事件（含目标会话与换行状态） |

---

## ⚡ 3. 拦截审查模式 (Interception Guard Pattern)

对于高危操作（如系统关键文件删除、未保存退出等），事件载荷内置原子取消标志与拦截原因，监听者可在监听回调中直接调用 `.abort(reason)`：

```rust
// 挂载高危文件删除防御监听
let guard = events.global().listen(|e: &FileOperationBeforeEvent| {
    if e.action == "delete" && !e.is_remote {
        let p = e.path.to_lowercase();
        if p == "/" || p == "c:\\" || p.contains("windows\\system32") {
            e.abort("禁止删除操作系统关键核心文件或根目录！");
        }
    }
});

// UI 层触发删除前先派发审查事件
let before_event = FileOperationBeforeEvent::new("delete", is_remote, &p_str);
core_state.events().dispatch(&before_event);
if before_event.is_aborted() {
    notify_warning("高危操作拦截", before_event.abort_reason().unwrap_or_default());
    return;
}
```

---

## 🔒 4. 并发安全性与防死锁设计

1. **临界区极小化**：`EventDispatcher::dispatch` 仅在获取读锁时快速 clone 当前事件的所有 listener 引用，随即立即释放读锁，随后在**完全无锁**状态下遍历执行回调函数。即使监听者在回调内部再次触发 `listen` 或 `dispatch`，也绝对不会造成死锁。
2. **Panic 隔离**：分发器通过 `std::panic::catch_unwind` 包装每个监听者的执行，单个监听者的意外崩溃会被捕获并记录错误日志，绝不影响主 UI 线程与其他监听者。
3. **RAII 自动反注册**：调用 `listen()` 返回 `ListenerGuard`，当持有者对象（如页面/组件）析构时，监听自动从分发器移除，杜绝悬挂闭包与内存泄露；全局常驻监听可调用 `.detach()` 脱离生命周期。

---

## 📋 5. 系统当前常驻运行的默认监听器清单 (Active Listeners Inventory)

在 `CoreState` 状态引擎初始化（[`attach_default_event_loggers`](file:///F:/code/rust/smalux-ssh/crates/smagical-core/src/state/core_state.rs#L32-L180)）时，系统会自动注册以下常驻日志与审计监听器：

| 序号 | 监听事件 (Event Type) | 日志级别 | Target | 监听器职责与输出内容 |
| :---: | :--- | :---: | :--- | :--- |
| **1** | `CredentialSavedEvent` | `INFO` | `smalux::credential` | 记录凭据新建/更新详情（ID、名称、算法、凭据类型、指纹、是否新建） |
| **2** | `CredentialDeletedEvent` | `WARN` | `smalux::credential` | 记录凭据物理删除告警 |
| **3** | `CredentialSecretCopiedEvent` | `WARN` / `INFO` | `smalux::security` | **安全审计守护**：监控密码/私钥复制动作（高危提取标记并告警） |
| **4** | `KeyGeneratedEvent` | `INFO` | `smalux::credential` | 记录新生成的密钥规格与公钥 SHA-256 指纹 |
| **5** | `PasswordGeneratedEvent` | `DEBUG` | `smalux::credential` | 记录强密码生成事件（物理脱敏，杜绝明文写入日志） |
| **6** | `FileOperationBeforeEvent` | `WARN` | `smalux::security` | **前置安全防御拦截器**：拦截删除系统根目录或 Windows/Linux 关键系统目录 |
| **7** | `TerminalSessionEvent` | `INFO` | `smalux::terminal` | 记录终端会话开启、关闭与连接切换 |
| **8** | `ConfigChangedEvent` | `INFO` | `smalux::config` | 记录全局系统配置参数热变动（键名、旧值、新值、变更来源） |
| **9** | `ThemeChangedEvent` | `INFO` | `smalux::theme` | 记录 UI/终端主题切换与深浅模式状态 |
| **10** | `WindowStateChangedEvent` | `INFO` | `smalux::window` | 记录窗口最小化、最大化、还原等状态流转 |
| **11** | `SnippetSavedEvent` | `INFO` | `smalux::snippet` | 记录代码片段新建/修改保存（ID、标题、是否新建） |
| **12** | `SnippetDeletedEvent` | `WARN` | `smalux::snippet` | 记录代码片段物理删除 |
| **13** | `SnippetExecutedEvent` | `INFO` | `smalux::snippet` | 记录代码片段向终端注入执行（ID、目标 session_id、自动回车状态） |
| **14** | `SnippetGroupSavedEvent` | `INFO` | `smalux::snippet` | 记录代码片段文件夹新建/重命名/移动（ID、名称、父级 ID） |
| **15** | `SnippetGroupDeletedEvent` | `WARN` | `smalux::snippet` | 记录代码片段文件夹物理删除 |

