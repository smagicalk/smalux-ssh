//! 终端输入回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::{LOCAL_TERMINAL_SESSION_ID, Message};

use super::{
    AppWindow, SharedAppState, apply_and_sync, apply_without_sync, parse_session_id,
    sync_terminal_pane,
};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
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
