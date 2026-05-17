//! 一次性远程命令和命令历史调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, PtyRequest, RemoteCommandRequest};
use crate::model::{
    CommandHistoryId, CommandHistoryItem, HostId, SessionId, SessionStatus, WorkspacePage,
};
use crate::terminal::TerminalTabState;

use super::launch::{connect_command, missing_host, queued_outcome, unix_now_secs};
use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 执行一次性远程命令，并记录主机作用域命令历史。
    pub(super) fn run_remote_command(
        &mut self,
        host_id: HostId,
        command: String,
        request_pty: bool,
    ) -> AppUpdateOutcome {
        let command = command.trim().to_owned();
        if command.is_empty() {
            return AppUpdateOutcome {
                error: Some("远程命令不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };

        let session_id = SessionId(Uuid::new_v4());
        let terminal_tab = TerminalTabState::new(session_id, command.clone());
        let request = remote_command_request(command.clone(), terminal_tab.size, request_pty);
        let history_id = self.record_command_history(host.id, command.clone());

        self.sessions.open_remote_command_tab(
            session_id,
            host.id,
            command.clone(),
            Some(history_id),
        );
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.ui.workspace.active_page = WorkspacePage::Terminal;
        self.terminal.open_tab(terminal_tab);
        self.record_recent_connection(&host);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::RunCommand {
                session_id,
                request,
            },
        ]);

        queued_outcome(2)
    }

    /// 重新执行一条带主机作用域的历史命令。
    pub(super) fn run_command_history(&mut self, history_id: CommandHistoryId) -> AppUpdateOutcome {
        let Some(history) = self
            .storage
            .command_history
            .iter()
            .find(|item| item.id == history_id)
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到命令历史：{}", history_id.0)),
                ..AppUpdateOutcome::default()
            };
        };

        let Some(host_id) = history.host_id else {
            return AppUpdateOutcome {
                error: Some("命令历史缺少主机，无法直接重跑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command = history.command.clone();

        self.run_remote_command(host_id, command, false)
    }

    pub(super) fn record_command_history(
        &mut self,
        host_id: HostId,
        command: String,
    ) -> CommandHistoryId {
        let history_id = CommandHistoryId(Uuid::new_v4());
        self.storage.add_command_history(CommandHistoryItem {
            id: history_id,
            host_id: Some(host_id),
            command,
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: unix_now_secs(),
            duration_ms: None,
        });
        history_id
    }
}

fn remote_command_request(
    command: String,
    size: crate::terminal::TerminalSize,
    request_pty: bool,
) -> RemoteCommandRequest {
    if request_pty {
        RemoteCommandRequest::with_pty(command, PtyRequest::xterm(size))
    } else {
        RemoteCommandRequest::exec(command)
    }
}
