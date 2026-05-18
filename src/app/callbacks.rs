//! Slint 回调到领域消息的绑定。

use std::rc::Rc;

use crate::model::Message;

use super::ids::{parse_command_history_id, parse_host_id, parse_session_id};
use super::projection::{sync_terminal_pane, sync_window};
use super::{AppWindow, SharedAppState};

/// 绑定所有 Slint 顶层回调。
pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    navigation::bind(window, Rc::clone(&state));
    workspace::bind(window, Rc::clone(&state));
    command_palette::bind(window, Rc::clone(&state));
    host_actions::bind(window, Rc::clone(&state));
    terminal_actions::bind(window, Rc::clone(&state));
    sftp_actions::bind(window, Rc::clone(&state));
    session_actions::bind(window, state);
}

mod command_palette;
mod host_actions;
mod navigation;
mod session_actions;
mod sftp_actions;
mod terminal_actions;
mod workspace;

fn apply_and_sync(weak: &slint::Weak<AppWindow>, state: &SharedAppState, message: Message) {
    apply_messages_and_sync(weak, state, [message]);
}

fn apply_without_sync(state: &SharedAppState, message: Message) {
    state.borrow_mut().apply(message);
}

fn apply_messages_and_sync<const N: usize>(
    weak: &slint::Weak<AppWindow>,
    state: &SharedAppState,
    messages: [Message; N],
) {
    let Some(window) = weak.upgrade() else {
        return;
    };

    {
        let mut state = state.borrow_mut();
        let storage_before = state.storage.clone();

        for message in messages {
            state.apply(message);
            state.drain_backend_queue_with_executor();
        }

        if state.storage != storage_before {
            if let Err(error) = state.persist_storage() {
                tracing::error!(error = %error, "保存本地存储失败");
                state
                    .ui
                    .set_last_error(format!("保存本地存储失败：{error}"));
            }
        }
    }

    sync_window(&window, &state.borrow());
}

#[allow(dead_code)]
fn parse_tool_panel_mode(mode: &str) -> Option<crate::model::ToolPanelMode> {
    match mode {
        "SFTP" => Some(crate::model::ToolPanelMode::Sftp),
        "Snippets" => Some(crate::model::ToolPanelMode::Snippets),
        "History" => Some(crate::model::ToolPanelMode::History),
        "Tunnels" => Some(crate::model::ToolPanelMode::Tunnels),
        "KnownHosts" => Some(crate::model::ToolPanelMode::KnownHosts),
        _ => None,
    }
}

#[allow(dead_code)]
fn active_terminal_host_id(state: &crate::model::AppState) -> Option<crate::model::HostId> {
    let session_id = state
        .terminal
        .active_tab
        .or_else(|| state.terminal.tabs.last().map(|tab| tab.session_id))?;

    state
        .sessions
        .tabs
        .iter()
        .find(|tab| tab.id == session_id)
        .and_then(|tab| tab.host_id)
}
