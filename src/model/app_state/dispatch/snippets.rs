//! 命令片段和历史命令消息路由。
//!
//! 这里处理命令模板的保存、变量填充、执行和历史命令重放。片段最终会变成
//! 远程命令启动请求，但模板渲染和变量校验留在 snippet 领域内。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 分发命令片段和历史命令消息。
    pub(super) fn dispatch_snippet_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::SaveHostCommandSnippet { host_id } => self.save_host_command_snippet(host_id),
            Message::RunSnippet {
                host_id,
                snippet_id,
            } => self.run_snippet(host_id, snippet_id),
            Message::UpdateSnippetArgument {
                snippet_id,
                name,
                value,
            } => self.update_snippet_argument(snippet_id, name, value),
            Message::RemoveSnippet { snippet_id } => self.remove_snippet(snippet_id),
            Message::RunCommandHistory { history_id } => self.run_command_history(history_id),
            _ => unreachable!("非命令片段消息不应进入命令片段路由"),
        }
    }
}
