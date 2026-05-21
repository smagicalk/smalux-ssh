//! 工作区快照消息路由。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn dispatch_workspace_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::SaveWorkspaceSnapshot => self.save_workspace_snapshot(),
            Message::RestoreWorkspaceSnapshot => self.restore_workspace_snapshot(),
            Message::ClearWorkspaceSnapshot => self.clear_workspace_snapshot(),
            _ => unreachable!("非工作区快照消息不应进入工作区快照路由"),
        }
    }
}
