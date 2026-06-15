//! Slint 回调到领域消息的绑定。
//!
//! 这层是当前 Slint UI 的“输入 Adapter”：
//!
//! - 从 Slint 回调接收字符串、布尔值和 UI 事件。
//! - 解析成核心 ID、枚举或 `Message`。
//! - 调用桌面状态入口提交 `Message`。
//! - 状态变化后调用 `projection::sync_window` 重新投影 UI。
//!
//! 这里允许知道 Slint 的回调名，但不应该实现核心业务规则。业务规则应该放在
//! `src/model/app_state/*`，这样重写 UI 时只需要替换 callback 绑定。

use std::rc::Rc;

use crate::model::Message;

use super::ids::{
    parse_command_history_id, parse_host_id, parse_optional_group_id, parse_session_id,
};
use super::projection::sync_window;
use super::state::AsDesktopStateView;
use super::{AppWindow, SharedAppState};

/// 绑定所有 Slint 顶层回调。
pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    navigation::bind(window, Rc::clone(&state));
    workspace::bind(window, Rc::clone(&state));
    command_palette::bind(window, Rc::clone(&state));
    host_actions::bind(window, Rc::clone(&state));
    settings_actions::bind(window, Rc::clone(&state));
    terminal_actions::bind(window, Rc::clone(&state));
    sftp_actions::bind(window, Rc::clone(&state));
    session_actions::bind(window, state);
}

mod command_palette;
mod host_actions;
mod host_actions_credentials;
mod host_actions_helpers;
mod host_actions_quick_host;
mod navigation;
mod session_actions;
mod settings_actions;
mod sftp_actions;
mod terminal_actions;
mod workspace;

fn apply_and_sync(weak: &slint::Weak<AppWindow>, state: &SharedAppState, message: Message) {
    apply_messages_and_sync(weak, state, [message]);
}

fn apply_and_sync_success(
    weak: &slint::Weak<AppWindow>,
    state: &SharedAppState,
    message: Message,
) -> bool {
    let Some(window) = weak.upgrade() else {
        return false;
    };

    let mut success = true;
    {
        let mut state = state.borrow_mut();
        let (outcome, persist_error) = state.apply_messages_with_persistence([message]);
        success &= outcome.error.is_none();

        if let Some(error) = persist_error {
            tracing::error!(error = %error, "保存本地存储失败");
            state
                .ui
                .set_last_error(format!("保存本地存储失败：{error}"));
            success = false;
        }
    }

    let state = state.borrow();
    sync_window(&window, state.as_desktop_state_view());
    success
}

/// 只提交消息，不立即刷新 Slint。
///
/// 用于高频输入或需要局部刷新路径的地方。调用者必须确保后续会用更小范围的
/// projection 同步 UI，否则界面会停留在旧状态。
fn apply_without_sync(state: &SharedAppState, message: Message) {
    state.borrow_mut().apply_message(message);
}

/// 提交单条消息并刷新 UI，但跳过后端 pump 相关处理。
///
/// 适合只影响本地 UI/存储的操作。这个函数仍会在存储快照变化时落盘。
fn apply_and_sync_without_drain(
    weak: &slint::Weak<AppWindow>,
    state: &SharedAppState,
    message: Message,
) {
    let Some(window) = weak.upgrade() else {
        return;
    };

    {
        let mut state = state.borrow_mut();
        let (_, persist_error) = state.apply_messages_with_persistence([message]);

        if let Some(error) = persist_error {
            tracing::error!(error = %error, "保存本地存储失败");
            state
                .ui
                .set_last_error(format!("保存本地存储失败：{error}"));
        }
    }

    let state = state.borrow();
    sync_window(&window, state.as_desktop_state_view());
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
        let (_, persist_error) = state.apply_messages_with_persistence(messages);

        if let Some(error) = persist_error {
            tracing::error!(error = %error, "保存本地存储失败");
            state
                .ui
                .set_last_error(format!("保存本地存储失败：{error}"));
        }
    }

    let state = state.borrow();
    sync_window(&window, state.as_desktop_state_view());
}

/// 把 Slint 传回来的工具面板 key 解析成核心枚举。
///
/// Slint 只传稳定 key，不传本地化文案。这样中文、英文或未来其他语言不会影响
/// 业务分发。
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

fn active_terminal_host_id(state: &crate::core::CoreState) -> Option<crate::model::HostId> {
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
