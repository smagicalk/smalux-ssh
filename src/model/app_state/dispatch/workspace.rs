//! 工作区快照消息路由。
//!
//! 工作区快照保存的是可恢复标签页和布局，不负责自动重新连接 SSH。恢复时只重建
//! 可见状态和本地结构，真实连接仍由用户显式触发。

use crate::core::CoreState;

use super::super::{AppState, AppUpdateOutcome, Message};

impl CoreState {
    /// 分发工作区快照核心消息。
    pub(super) fn dispatch_workspace_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::SaveWorkspaceSnapshot => self.save_workspace_snapshot(),
            Message::RestoreWorkspaceSnapshot => self.restore_workspace_snapshot(),
            Message::ClearWorkspaceSnapshot => self.clear_workspace_snapshot(),
            _ => unreachable!("非工作区快照消息不应进入工作区快照路由"),
        }
    }
}

impl AppState {
    /// 分发工作区快照消息。
    pub(super) fn dispatch_workspace_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::SaveWorkspaceSnapshot
            | Message::RestoreWorkspaceSnapshot
            | Message::ClearWorkspaceSnapshot => self.core.dispatch_workspace_message(message),
            _ => unreachable!("非工作区快照消息不应进入工作区快照路由"),
        }
    }
}
