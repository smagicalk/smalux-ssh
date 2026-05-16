//! Shell 会话启动和共享的后端命令调度辅助。

use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::backend::{BackendCommand, ConnectionTarget, PtyRequest};
use crate::model::{Host, HostId, RecentConnection, SessionId, SessionStatus, WorkspacePage};
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
        self.ui.workspace.active_page = WorkspacePage::Terminal;
        self.terminal.open_tab(terminal_tab);
        self.record_recent_connection(&host);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::OpenShell { session_id, pty },
        ]);

        queued_outcome(2)
    }

    /// 从最近连接记录重新打开交互式 Shell。
    pub(super) fn open_recent_connection(&mut self, host_id: HostId) -> AppUpdateOutcome {
        self.open_shell(host_id)
    }

    pub(super) fn host_by_id(&self, host_id: HostId) -> Option<Host> {
        self.storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
    }

    pub(super) fn record_recent_connection(&mut self, host: &Host) {
        self.storage.record_recent_connection(RecentConnection {
            host_id: host.id,
            label: host.name.clone(),
            connected_at_unix_secs: unix_now_secs(),
        });
    }
}

pub(super) fn connect_command(session_id: SessionId, host: &Host) -> BackendCommand {
    BackendCommand::Connect {
        session_id,
        target: ConnectionTarget::from_host(host),
    }
}

pub(super) fn queued_outcome(queued_backend_commands: usize) -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        queued_backend_commands,
        ..AppUpdateOutcome::default()
    }
}

pub(super) fn missing_host(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

pub(super) fn normalize_remote_dir(remote_dir: &str) -> String {
    let remote_dir = remote_dir.trim();

    if remote_dir.is_empty() {
        "/".to_owned()
    } else {
        remote_dir.to_owned()
    }
}

pub(super) fn join_remote_path(remote_dir: &str, name: &str) -> String {
    if remote_dir == "/" {
        format!("/{name}")
    } else {
        format!(
            "{}/{}",
            remote_dir.trim_end_matches('/'),
            name.trim_start_matches('/')
        )
    }
}

pub(super) fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
