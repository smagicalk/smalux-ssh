//! 应用消息分发。
//!
//! 这里只负责把 `Message` 路由到具体领域模块，避免根状态文件承担巨大的匹配分发。

#[path = "dispatch/backend.rs"]
mod backend;
#[path = "dispatch/launch.rs"]
mod launch;
#[path = "dispatch/session.rs"]
mod session;
#[path = "dispatch/sftp.rs"]
mod sftp;
#[path = "dispatch/snippets.rs"]
mod snippets;
#[path = "dispatch/storage.rs"]
mod storage;
#[path = "dispatch/target.rs"]
mod target;
#[path = "dispatch/ui.rs"]
mod ui;
#[path = "dispatch/visual.rs"]
mod visual;
#[path = "dispatch/workspace.rs"]
mod workspace;

use super::{AppState, AppUpdateOutcome, Message};
use target::MessageDispatchTarget;

impl AppState {
    /// 将 UI 消息应用到根状态。
    pub fn apply(&mut self, message: Message) -> AppUpdateOutcome {
        let mut outcome = self.dispatch_message(message);

        if let Some(error) = &outcome.error {
            outcome.state_changed |= self.ui.set_last_error(error.clone());
        }

        outcome
    }

    fn dispatch_message(&mut self, message: Message) -> AppUpdateOutcome {
        match MessageDispatchTarget::for_message(&message) {
            MessageDispatchTarget::Visual => self.dispatch_visual_message(message),
            MessageDispatchTarget::Workspace => self.dispatch_workspace_message(message),
            MessageDispatchTarget::Ui => self.dispatch_ui_message(message),
            MessageDispatchTarget::Storage => self.dispatch_storage_message(message),
            MessageDispatchTarget::Session => self.dispatch_session_message(message),
            MessageDispatchTarget::Sftp => self.dispatch_sftp_message(message),
            MessageDispatchTarget::Launch => self.dispatch_launch_message(message),
            MessageDispatchTarget::Snippet => self.dispatch_snippet_message(message),
            MessageDispatchTarget::Backend => self.dispatch_backend_message(message),
        }
    }

    fn dismiss_ui_error(&mut self) -> AppUpdateOutcome {
        AppUpdateOutcome {
            state_changed: self.ui.clear_last_error(),
            ..AppUpdateOutcome::default()
        }
    }
}
