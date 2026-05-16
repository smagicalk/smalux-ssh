//! 后端命令队列。

use std::collections::VecDeque;

use super::BackendCommand;

/// UI 状态层等待提交给后端执行器的命令队列。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackendCommandQueue {
    commands: VecDeque<BackendCommand>,
}

impl BackendCommandQueue {
    /// 追加一条待执行命令。
    pub fn push(&mut self, command: BackendCommand) {
        self.commands.push_back(command);
    }

    /// 按原始顺序追加多条待执行命令。
    pub fn extend(&mut self, commands: impl IntoIterator<Item = BackendCommand>) {
        self.commands.extend(commands);
    }

    /// 弹出最早入队的命令。
    pub fn pop_front(&mut self) -> Option<BackendCommand> {
        self.commands.pop_front()
    }

    /// 查看最早入队的命令，不消费队列。
    pub fn front(&self) -> Option<&BackendCommand> {
        self.commands.front()
    }

    /// 排空队列并返回待执行命令。
    pub fn drain(&mut self) -> Vec<BackendCommand> {
        self.commands.drain(..).collect()
    }

    /// 保留满足条件的命令，并返回被移除的命令数量。
    pub fn retain(&mut self, mut keep: impl FnMut(&BackendCommand) -> bool) -> usize {
        let before = self.commands.len();
        self.commands.retain(|command| keep(command));
        before - self.commands.len()
    }

    /// 当前等待执行的命令数量。
    pub fn pending_count(&self) -> usize {
        self.commands.len()
    }

    /// 队列是否为空。
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{PtyRequest, RemoteCommandRequest};
    use crate::model::SessionId;
    use crate::terminal::TerminalSize;
    use uuid::Uuid;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    #[test]
    fn queue_preserves_backend_command_order() {
        let mut queue = BackendCommandQueue::default();
        let session_id = session_id();

        queue.extend([
            BackendCommand::OpenShell {
                session_id,
                pty: PtyRequest::xterm(TerminalSize::default()),
            },
            BackendCommand::RunCommand {
                session_id,
                request: RemoteCommandRequest::exec("uptime"),
            },
        ]);

        assert_eq!(queue.pending_count(), 2);
        assert!(matches!(
            queue.front(),
            Some(BackendCommand::OpenShell { .. })
        ));
        assert!(matches!(
            queue.pop_front(),
            Some(BackendCommand::OpenShell { .. })
        ));
        assert!(matches!(
            queue.pop_front(),
            Some(BackendCommand::RunCommand { .. })
        ));
        assert!(queue.is_empty());
    }

    #[test]
    fn drain_returns_all_commands_and_clears_queue() {
        let mut queue = BackendCommandQueue::default();
        let session_id = session_id();

        queue.push(BackendCommand::Disconnect { session_id });

        let commands = queue.drain();

        assert_eq!(commands.len(), 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn retain_removes_matching_commands_and_reports_count() {
        let mut queue = BackendCommandQueue::default();
        let first = session_id();
        let second = session_id();

        queue.extend([
            BackendCommand::Disconnect { session_id: first },
            BackendCommand::Disconnect { session_id: second },
        ]);

        let removed = queue.retain(|command| command.session_id() != first);

        assert_eq!(removed, 1);
        assert_eq!(queue.pending_count(), 1);
        assert!(matches!(
            queue.front(),
            Some(BackendCommand::Disconnect { session_id }) if *session_id == second
        ));
    }
}
