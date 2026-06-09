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
#[path = "dispatch/ui.rs"]
mod ui;
#[path = "dispatch/ui_quick_host.rs"]
mod ui_quick_host;
#[path = "dispatch/ui_terminal.rs"]
mod ui_terminal;
#[path = "dispatch/ui_workspace.rs"]
mod ui_workspace;
#[path = "dispatch/visual.rs"]
mod visual;
#[path = "dispatch/workspace.rs"]
mod workspace;

use super::{AppState, AppUpdateOutcome, Message};
use target::MessageDispatchTarget;

impl AppState {
    /// 将 UI 消息应用到根状态。
    ///
    /// 这是核心层唯一推荐给 UI Adapter 调用的状态变更入口。调用者不应该直接改
    /// `AppState` 字段，否则会绕过错误记录、后端命令排队、存储落盘和测试覆盖。
    pub fn apply(&mut self, message: Message) -> AppUpdateOutcome {
        let mut outcome = self.dispatch_message(message);

        // 所有领域函数只返回错误文本，不直接操作错误提示 UI。这里统一把错误写入
        // `ui.last_error`，让 Slint 或其他 UI 只需要渲染一个错误出口。
        if let Some(error) = &outcome.error {
            outcome.state_changed |= self.ui.set_last_error(error.clone());
        }

        outcome
    }

    /// 根据消息类别把消息转给对应领域路由。
    ///
    /// 这里保持二级分发，而不是写一个超大的 `match Message`，是为了让每个功能域
    /// 的新增、测试和删除都有局部性。
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

    /// 清除当前 UI 错误提示。
    ///
    /// 这是少数直接修改 UI 草稿的公共入口，保留在 dispatch 根模块中可以避免为了
    /// 一个简单动作单独创建领域文件。
    fn dismiss_ui_error(&mut self) -> AppUpdateOutcome {
        AppUpdateOutcome {
            state_changed: self.ui.clear_last_error(),
            ..AppUpdateOutcome::default()
        }
    }
}
