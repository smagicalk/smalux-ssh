//! 工作区布局与侧边栏回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::Message;

use super::{AppWindow, SharedAppState, apply_and_sync};

#[path = "workspace/known_hosts.rs"]
mod known_hosts;
#[path = "workspace/layout.rs"]
mod layout;
#[path = "workspace/tool_panel.rs"]
mod tool_panel;

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    layout::bind(window, &state);

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
}
