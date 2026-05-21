//! 命令片段和历史命令消息路由。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
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
