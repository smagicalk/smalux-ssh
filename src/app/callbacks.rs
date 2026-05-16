//! Slint 回调到领域消息的绑定。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::{
    DEFAULT_REMOTE_COMMAND, HostId, LOCAL_TERMINAL_SESSION_ID, Message, ToolPanelMode,
    WorkspacePage,
};

use super::ids::{parse_command_history_id, parse_host_id, parse_session_id};
use super::projection::{sync_terminal_pane, sync_window};
use super::{AppWindow, SharedAppState};

/// 绑定所有 Slint 顶层回调。
pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    bind_navigation(window, Rc::clone(&state));
    bind_workspace_actions(window, Rc::clone(&state));
    bind_command_palette(window, Rc::clone(&state));
    bind_host_actions(window, Rc::clone(&state));
    bind_terminal_actions(window, Rc::clone(&state));
    bind_sftp_actions(window, Rc::clone(&state));
    bind_session_actions(window, state);
}

fn bind_navigation(window: &AppWindow, state: SharedAppState) {
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Hosts,
        WorkspacePage::Hosts,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Terminal,
        WorkspacePage::Terminal,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Sftp,
        WorkspacePage::Sftp,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Tunnels,
        WorkspacePage::Tunnels,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Snippets,
        WorkspacePage::Snippets,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::History,
        WorkspacePage::History,
    );
    bind_page(
        window,
        state,
        WindowCallback::Settings,
        WorkspacePage::Settings,
    );
}

fn bind_workspace_actions(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_toggle_host_list_mode(move || {
            apply_and_sync(&weak, &state, Message::ToggleHostListMode);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_host_search(move |query| {
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateHostSearchQuery {
                    query: query.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_toggle_right_sidebar(move || {
            apply_and_sync(&weak, &state, Message::ToggleRightSidebar);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_resize_hosts_panel(move |width| {
            apply_and_sync(&weak, &state, Message::ResizeHostsPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_resize_activity_panel(move |width| {
            apply_and_sync(&weak, &state, Message::ResizeActivityPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_resize_tool_panel(move |width| {
            apply_and_sync(&weak, &state, Message::ResizeToolPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_tool_panel(move |mode| {
            let Some(mode) = parse_tool_panel_mode(&mode) else {
                return;
            };
            let host_without_sftp = if matches!(mode, ToolPanelMode::Sftp) {
                let state = state.borrow();
                active_terminal_host_id(&state).filter(|host_id| {
                    !state
                        .sessions
                        .sftp_browsers
                        .iter()
                        .any(|browser| browser.host_id == *host_id)
                })
            } else {
                None
            };

            if let Some(host_id) = host_without_sftp {
                apply_messages_and_sync(
                    &weak,
                    &state,
                    [
                        Message::OpenSftp {
                            host_id,
                            initial_dir: "/".to_owned(),
                        },
                        Message::OpenToolPanel { mode },
                    ],
                );
            } else {
                apply_and_sync(&weak, &state, Message::OpenToolPanel { mode });
            }
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_close_tool_panel(move || {
            apply_and_sync(&weak, &state, Message::CloseToolPanel);
        });
    }
    {
        let weak = window.as_weak();
        window.on_next_background(move || {
            apply_and_sync(&weak, &state, Message::NextBackground);
        });
    }
}

fn parse_tool_panel_mode(mode: &str) -> Option<ToolPanelMode> {
    match mode {
        "SFTP" => Some(ToolPanelMode::Sftp),
        "Snippets" => Some(ToolPanelMode::Snippets),
        "History" => Some(ToolPanelMode::History),
        "Tunnels" => Some(ToolPanelMode::Tunnels),
        _ => None,
    }
}

fn active_terminal_host_id(state: &crate::model::AppState) -> Option<HostId> {
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

fn bind_command_palette(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_command_palette(move || {
            apply_and_sync(
                &weak,
                &state,
                Message::OpenCommandPalette {
                    query: String::new(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_command_palette(move |query| {
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateCommandPaletteQuery {
                    query: query.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_close_command_palette(move || {
            apply_and_sync(&weak, &state, Message::CloseCommandPalette);
        });
    }
    {
        let weak = window.as_weak();
        window.on_activate_command_palette_item(move |item_id, kind| {
            let message = match kind.as_str() {
                "Host" | "Recent" => {
                    parse_host_id(&item_id).map(|host_id| Message::OpenShell { host_id })
                }
                "History" => parse_command_history_id(&item_id)
                    .map(|history_id| Message::RunCommandHistory { history_id }),
                _ => None,
            };

            if let Some(message) = message {
                apply_messages_and_sync(&weak, &state, [message, Message::CloseCommandPalette]);
            }
        });
    }
}

fn bind_host_actions(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_shell(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::OpenShell { host_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_host_sftp(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_messages_and_sync(
                &weak,
                &state,
                [
                    Message::OpenSftp {
                        host_id,
                        initial_dir: "/".to_owned(),
                    },
                    Message::OpenToolPanel {
                        mode: ToolPanelMode::Sftp,
                    },
                ],
            );
        });
    }
    {
        let weak = window.as_weak();
        window.on_run_host_command(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::RunRemoteCommand {
                    host_id,
                    command: DEFAULT_REMOTE_COMMAND.to_owned(),
                    request_pty: true,
                },
            );
        });
    }
}

fn bind_terminal_actions(window: &AppWindow, state: SharedAppState) {
    {
        let state = Rc::clone(&state);
        window.on_update_terminal_input(move |session_id, text| {
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            // TextInput 的 edited 会逐字触发；这里不能同步整窗模型，否则输入会明显卡顿。
            apply_without_sync(
                &state,
                Message::UpdateTerminalInputDraft {
                    session_id,
                    input: text.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_append_terminal_input(move |session_id, text| {
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::AppendTerminalInputDraft {
                    session_id,
                    text: text.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_backspace_terminal_input(move |session_id| {
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::BackspaceTerminalInputDraft { session_id },
            );
        });
    }
    {
        let weak = window.as_weak();
        window.on_send_terminal_input(move |session_id, text| {
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            {
                let mut state = state.borrow_mut();
                state.apply(Message::UpdateTerminalInputDraft {
                    session_id,
                    input: text.to_string(),
                });
                state.apply(Message::SendTerminalInput { session_id });

                if session_id == LOCAL_TERMINAL_SESSION_ID {
                    state.drain_backend_queue_with_executor();
                }
            }

            let Some(window) = weak.upgrade() else {
                return;
            };
            sync_terminal_pane(&window, &state.borrow());
        });
    }
}

fn bind_sftp_actions(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_refresh_sftp(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::RefreshSftp { host_id });
        });
    }
    {
        let weak = window.as_weak();
        window.on_select_sftp_entry(move |host_id, path| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::SelectSftpEntry {
                    host_id,
                    remote_path: path.to_string(),
                },
            );
        });
    }
}

fn bind_session_actions(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_activate_session(move |session_id| {
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::ActivateTerminalTab { session_id });
        });
    }
    {
        let weak = window.as_weak();
        window.on_close_session(move |session_id| {
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::CloseSessionTab { session_id });
        });
    }
}

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

fn bind_page(
    window: &AppWindow,
    state: SharedAppState,
    callback: WindowCallback,
    page: WorkspacePage,
) {
    let weak = window.as_weak();
    let handler = move || {
        apply_and_sync(&weak, &state, Message::SetWorkspacePage { page });
    };

    match callback {
        WindowCallback::Hosts => window.on_open_hosts(handler),
        WindowCallback::Terminal => window.on_open_terminal(handler),
        WindowCallback::Sftp => window.on_open_sftp(handler),
        WindowCallback::Tunnels => window.on_open_tunnels(handler),
        WindowCallback::Snippets => window.on_open_snippets(handler),
        WindowCallback::History => window.on_open_history(handler),
        WindowCallback::Settings => window.on_open_settings(handler),
    }
}

enum WindowCallback {
    Hosts,
    Terminal,
    Sftp,
    Tunnels,
    Snippets,
    History,
    Settings,
}
