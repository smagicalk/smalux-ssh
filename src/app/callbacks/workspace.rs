//! 工作区布局与侧边栏回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::{Message, ToolPanelMode};

use super::{
    AppWindow, SharedAppState, active_terminal_host_id, apply_and_sync, apply_messages_and_sync,
};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
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
            let Some(mode) = super::parse_tool_panel_mode(&mode) else {
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
        let state = Rc::clone(&state);
        window.on_trust_known_host(move |host, port| {
            let Some(port) = known_host_port(port) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::TrustKnownHost {
                    host: host.to_string(),
                    port,
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_remove_known_host(move |host, port| {
            let Some(port) = known_host_port(port) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::RemoveKnownHost {
                    host: host.to_string(),
                    port,
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        window.on_next_background(move || {
            apply_and_sync(&weak, &state, Message::NextBackground);
        });
    }
}

fn known_host_port(port: i32) -> Option<u16> {
    u16::try_from(port).ok().filter(|port| *port > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_host_port_rejects_invalid_values() {
        assert_eq!(known_host_port(22), Some(22));
        assert_eq!(known_host_port(0), None);
        assert_eq!(known_host_port(-1), None);
        assert_eq!(known_host_port(70_000), None);
    }
}
