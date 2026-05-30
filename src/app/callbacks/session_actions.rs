//! 会话管理回调。
//!
//! 会话 Tab 的激活、关闭、重连都通过 session_id 字符串回到核心层。UI 不直接删除
//! terminal buffer，也不判断连接状态，避免和 `SessionManager` 的规则分叉。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::Message;

use super::{AppWindow, SharedAppState, apply_and_sync, parse_session_id};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_activate_session(move |session_id| {
            // 单击 tab 只激活，不触发连接或其他副作用。
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::ActivateTerminalTab { session_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_close_session(move |session_id| {
            // 关闭行为由核心层负责级联清理 session、terminal、SFTP 和待执行命令。
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::CloseSessionTab { session_id });
        });
    }
    {
        let weak = window.as_weak();
        window.on_reconnect_shell(move |session_id| {
            // 重连只对核心认为可重连的 shell 生效；按钮可见性由 view_model 控制。
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::ReconnectShell { session_id });
        });
    }
}
