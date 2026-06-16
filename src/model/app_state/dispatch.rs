//! 应用消息分发。
//!
//! 这里只负责把 `Message` 路由到具体领域模块，避免根状态文件承担巨大的匹配分发。
//! 新功能通常先新增 `Message`，再在 `target::MessageDispatchTarget` 里指定归属，
//! 最后把具体行为放进对应领域模块。

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
#[path = "dispatch/workspace.rs"]
mod workspace;

use crate::core::CoreState;

use super::{AppUpdateOutcome, Message};
use target::MessageDispatchTarget;

impl CoreState {
    /// 只应用不依赖桌面草稿的核心消息。
    ///
    /// 仍依赖输入框、弹窗或页面状态的消息由具体 UI Adapter 处理；这里仅接收
    /// 能在 `CoreState` 内独立完成的消息。
    pub fn apply_core_message(&mut self, message: Message) -> AppUpdateOutcome {
        match MessageDispatchTarget::for_message(&message) {
            MessageDispatchTarget::Backend => self.dispatch_backend_message(message),
            MessageDispatchTarget::Session => self.dispatch_session_message(message),
            MessageDispatchTarget::Sftp => self.dispatch_sftp_message(message),
            MessageDispatchTarget::Storage => self.dispatch_storage_message(message),
            MessageDispatchTarget::Snippet => self.dispatch_snippet_message(message),
            MessageDispatchTarget::Workspace => self.dispatch_workspace_message(message),
            MessageDispatchTarget::Launch => self.dispatch_launch_message(message),
            MessageDispatchTarget::Ui | MessageDispatchTarget::Visual => AppUpdateOutcome {
                error: Some("当前消息仍依赖桌面草稿状态，不能只在 CoreState 中运行".to_owned()),
                ..AppUpdateOutcome::default()
            },
        }
    }
}
