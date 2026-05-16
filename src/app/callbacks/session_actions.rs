//! 会话管理回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::Message;

use super::{AppWindow, SharedAppState, apply_and_sync, parse_session_id};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
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
