//! 会话标签页消息路由。
//!
//! 这里处理已经存在的标签页：激活、关闭以及关闭时的 SFTP/隧道/后端命令清理。
//! 新建标签页不在这里，而是在 launch 路由里完成。

use super::super::{AppUpdateOutcome, Message};
use crate::core::CoreState;

impl CoreState {
    /// 分发会话标签页消息。
    pub(super) fn dispatch_session_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::CloseSessionTab { session_id } => self.close_session_tab(session_id),
            Message::ActivateTerminalTab { session_id } => self.activate_session_tab(session_id),
            _ => unreachable!("非会话标签页消息不应进入会话标签页路由"),
        }
    }
}
