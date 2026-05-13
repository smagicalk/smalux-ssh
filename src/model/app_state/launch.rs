//! 会话启动和后端命令调度。

use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::backend::{BackendCommand, ConnectionTarget, PtyRequest, RemoteCommandRequest};
use crate::model::{
    CommandHistoryId, CommandHistoryItem, Host, HostId, RecentConnection, SessionId, SessionStatus,
};
use crate::terminal::TerminalTabState;

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 打开交互式 Shell，并把连接和 PTY 请求排入后端队列。
    pub(super) fn open_shell(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };

        let session_id = SessionId(Uuid::new_v4());
        let terminal_tab = TerminalTabState::new(session_id, host.name.clone());
        let pty = PtyRequest::xterm(terminal_tab.size);

        self.sessions
            .open_shell_tab(session_id, host.id, host.name.clone());
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.terminal.open_tab(terminal_tab);
        self.record_recent_connection(&host);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::OpenShell { session_id, pty },
        ]);

        queued_outcome(2)
    }

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

        self.sessions
            .open_remote_command_tab(session_id, host.id, command.clone());
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.terminal.open_tab(terminal_tab);
        self.record_recent_connection(&host);
        self.record_command_history(host.id, command);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::RunCommand {
                session_id,
                request,
            },
        ]);

        queued_outcome(2)
    }

    fn host_by_id(&self, host_id: HostId) -> Option<Host> {
        self.storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
    }

    fn record_recent_connection(&mut self, host: &Host) {
        self.storage.record_recent_connection(RecentConnection {
            host_id: host.id,
            label: host.name.clone(),
            connected_at_unix_secs: unix_now_secs(),
        });
    }

    fn record_command_history(&mut self, host_id: HostId, command: String) {
        self.storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command,
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: unix_now_secs(),
            duration_ms: None,
        });
    }
}

fn connect_command(session_id: SessionId, host: &Host) -> BackendCommand {
    BackendCommand::Connect {
        session_id,
        target: ConnectionTarget::from_host(host),
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

fn queued_outcome(queued_backend_commands: usize) -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        queued_backend_commands,
        ..AppUpdateOutcome::default()
    }
}

fn missing_host(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
