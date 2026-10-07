# plugin-history: 连接审计历史与终端屏幕快照回溯插件

> **模块定位**：Smalux 会话审计与历史时间流中心（左侧活动栏第六位）。负责记录每一次 SSH / 本地终端会话的完整生命周期（连接时间、时长、退出码、异常原因）、提供全屏审计时间轴 (`HistoryCenterView`)、侧边快速历史抽屉 (`HistoryDrawer`)、以及**终端最终视口快照回溯与一键重新连接 (`HistoryDetailModal`)**。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **会话生命周期全景审计 (`views/history_center_view.slint`)**：
   - 按时间流（今天、昨天、更早）分组汇总会话历史；
   - 记录会话类型（SSH、本地 PowerShell/CMD/WSL/Bash）、目标地址、用户名、连接起止时间与连接次数；
   - 标记会话退出状态：正常断开 (`exit`)、异常崩溃 (`error`)、网络超时中断 (`warning`)。
2. **终端最终屏幕快照回溯 (`modals/history_detail_modal.slint`)**：
   - 会话结束瞬间自动捕获终端最后 100 行可见内容并持久化缓存；
   - 随时点击查看历史会话当时的报错输出，杜绝“断线后现场丢失”的痛点；
   - 支持从快照卡片中点击“重新连接”，一键拉起新会话并自动切换至终端视口。
3. **快速历史抽屉 (`drawers/history_drawer.slint`)**：
   - 便于在终端中快速查看最近连接过的主机，点击即可在新标签页建立连接。

---

## 二、 核心 Slint 组件与调用方法 (Slint Components & Usage)

### 1. `HistoryCenterView` (全屏历史审计中心)
```slint
import { HistoryCenterView } from "@plugin-history/history_plugin.slint";

HistoryCenterView {
    horizontal-stretch: 1;
    reconnect(id) => {
        WindowBridge.main-view = "terminal";
        HistoryBridge.reconnect(id);
    }
}
```

### 2. `HistoryDetailModal` (终端屏幕快照与详情弹窗)
```slint
import { HistoryDetailModal } from "@plugin-history/history_plugin.slint";

HistoryDetailModal {
    close => { HistoryBridge.is-detail-open = false; }
    reconnect(id) => {
        HistoryBridge.is-detail-open = false;
        WindowBridge.main-view = "terminal";
        HistoryBridge.reconnect(id);
    }
}
```

---

## 三、 核心 Rust 驱动与 API 接口 (Rust Handlers & APIs)

在 `crates/smagical-ui/src/handlers/history_handlers.rs` 中集中管理：

### 1. 会话结束自动审计归档
```rust
pub async fn record_session_exit(
    session_id: &str,
    exit_status: &str,
    error_msg: Option<&str>,
    snapshot_text: &str,
    storage: &Arc<dyn AppStorage>,
) -> Result<()>
```
- **参数**：会话 ID、退出状态（如 `"exited"` / `"disconnected"`）、错误原因、最后屏幕纯文本内容与仓储接口；
- **说明**：自动将审计快照写入 SQLite WAL 库，并更新内存缓存 `HISTORY_CACHE`。

### 2. 检索历史记录 (ASCII 零分配快径)
```rust
pub fn filter_history_records(
    groups: &[HistoryGroupData],
    query: &str,
) -> Vec<HistoryGroupData>
```
- **说明**：通过 `matches_any_ignore_case` 对会话标题、地址与用户名进行毫秒级过滤。

---

## 四、 核心数据模型 (Data Models)

```rust
// 对应 Slint 中的 HistoryItemData
pub struct HistoryItemData {
    pub id: SharedString,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub address: SharedString,
    pub username: SharedString,
    pub session_type: SharedString, // "ssh" | "local"
    pub time_text: SharedString,
    pub duration_text: SharedString,
    pub exit_status: SharedString, // "online" | "warning" | "error" | "offline"
    pub error_msg: SharedString,
    pub is_pinned: bool,
    pub connect_count: i32,
    pub host_id: SharedString,
}
```
