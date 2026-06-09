//! 工作区布局与侧边栏回调。
//!
//! 工作区回调覆盖布局开关、工具面板、主题切换和 Known Hosts 操作。这里仍然只做事件
//! 转发；布局尺寸、面板状态和安全记录都由核心状态保存。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::Message;

use super::{AppWindow, SharedAppState, apply_and_sync};

#[path = "workspace/known_hosts.rs"]
mod known_hosts;
#[path = "workspace/layout.rs"]
mod layout;
#[path = "workspace/snippet_actions.rs"]
mod snippet_actions;
#[path = "workspace/snippet_helpers.rs"]
mod snippet_helpers;
#[path = "workspace/tool_panel.rs"]
mod tool_panel;

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    // 布局相关回调单独拆分，避免主工作区绑定文件继续膨胀。
    layout::bind(window, &state);
    snippet_actions::bind(window, &state);

    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_local_terminal(move || {
            // 本地终端和远程 shell 共享核心 session/terminal 模型。
            apply_and_sync(&weak, &state, Message::OpenLocalTerminal);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_tool_panel(move |mode| {
            // 工具面板 key 的解析集中在 tool_panel 子模块，保持这里只组织绑定。
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
        window.on_next_theme(move || {
            apply_and_sync(&weak, &state, Message::NextTheme);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_trust_known_host(move |host, port| {
            // Known Hosts 端口来自 UI 数值，仍需要子模块校验范围和构造消息。
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
        let state = Rc::clone(&state);
        window.on_dismiss_ui_error(move || {
            apply_and_sync(&weak, &state, Message::DismissUiError);
        });
    }
}
