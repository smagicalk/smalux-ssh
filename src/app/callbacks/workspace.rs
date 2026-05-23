//! 工作区布局与侧边栏回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::Message;

use super::{AppWindow, SharedAppState, apply_and_sync};

#[path = "workspace/known_hosts.rs"]
mod known_hosts;
#[path = "workspace/tool_panel.rs"]
mod tool_panel;

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
            tool_panel::open_tool_panel(&weak, &state, &mode);
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
            let Some(message) = known_hosts::trust_known_host_message(&host, port) else {
                return;
            };
            apply_and_sync(&weak, &state, message);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_remove_known_host(move |host, port| {
            let Some(message) = known_hosts::remove_known_host_message(&host, port) else {
                return;
            };
            apply_and_sync(&weak, &state, message);
        });
    }
    {
        let weak = window.as_weak();
        window.on_next_background(move || {
            apply_and_sync(&weak, &state, Message::NextBackground);
        });
    }
}
