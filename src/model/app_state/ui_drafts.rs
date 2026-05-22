//! UI 输入草稿消息处理。

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

#[cfg(test)]
pub(super) use local_terminal::ensure_local_terminal_tab;

pub(super) fn draft_changed() -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}
