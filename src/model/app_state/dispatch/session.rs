//! 会话标签页消息路由。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn dispatch_session_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::CloseSessionTab { session_id } => self.close_session_tab(session_id),
            Message::ActivateTerminalTab { session_id } => self.activate_session_tab(session_id),
            _ => unreachable!("非会话标签页消息不应进入会话标签页路由"),
        }
    }
}
