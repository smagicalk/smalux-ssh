//! 终端输入回调。
//!
//! 终端输入是高频路径。普通编辑只更新核心草稿，不刷新整个窗口；发送时再做完整提交并
//! 局部同步终端面板，避免每输入一个字符都重建大量 Slint 列表模型。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::app::projection::sync_terminal_pane;
use crate::model::Message;

use super::{AppWindow, SharedAppState, apply_and_sync, apply_without_sync, parse_session_id};

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
            // 追加输入通常来自键盘事件，需要经过核心过滤控制字符。
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
            // Backspace 同样由核心处理，保证 UTF-8 字符边界和草稿状态一致。
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
                // 发送前先写入最新文本，覆盖可能尚未触发 edited 的输入框内容。
                state.apply(Message::UpdateTerminalInputDraft {
                    session_id,
                    input: text.to_string(),
                });
                // SendTerminalInput 会校验会话、写历史、排队后端输入并清空草稿。
                state.apply(Message::SendTerminalInput { session_id });
            }

            let Some(window) = weak.upgrade() else {
                return;
            };
            // 只同步终端面板，避免回车后整窗列表重建导致输入体验抖动。
            sync_terminal_pane(&window, &state.borrow());
        });
    }
}
