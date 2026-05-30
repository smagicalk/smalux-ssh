//! UI 输入草稿消息处理。
//!
//! 草稿模块是 UI 和核心状态之间的缓冲层：输入框、弹窗、树选择等临时内容先写入
//! `UiState`，只有用户点击保存/发送时才转换成主机、分组、命令或后端请求。这样可以让
//! UI 组件重写时复用同一套状态和验证逻辑。

#[path = "ui_drafts/host_inputs.rs"]
mod host_inputs;
#[path = "ui_drafts/local_terminal.rs"]
mod local_terminal;
#[path = "ui_drafts/quick_host.rs"]
mod quick_host;
#[path = "ui_drafts/terminal_input.rs"]
mod terminal_input;
#[path = "ui_drafts/terminal_input_send.rs"]
mod terminal_input_send;

use super::AppUpdateOutcome;

pub(super) fn draft_changed() -> AppUpdateOutcome {
    // 草稿变化只代表需要重新投影 UI，不会排队后端命令，也不会修改业务数据。
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}
