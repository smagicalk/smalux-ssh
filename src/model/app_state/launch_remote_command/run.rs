//! 一次性远程命令启动。

use uuid::Uuid;

use crate::backend::BackendCommand;
use crate::core::CoreState;
use crate::model::{HostId, SessionId, SessionStatus};
use crate::terminal::TerminalTabState;

use super::super::AppUpdateOutcome;
use super::super::launch::{connect_command_with_known_hosts, missing_host, queued_outcome};
use super::request::remote_command_request;

impl CoreState {
    /// 执行一次性远程命令，并记录主机作用域命令历史。
    pub(crate) fn run_remote_command(
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
        self.terminal.open_tab(terminal_tab);
        self.record_recent_connection(&host);
        let known_hosts = self.storage.known_hosts.clone();
        self.backend_commands.extend([
            connect_command_with_known_hosts(session_id, &host, known_hosts),
            BackendCommand::RunCommand {
                session_id,
                request,
            },
        ]);

        queued_outcome(2)
    }
}
