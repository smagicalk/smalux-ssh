//! 交互式 Shell 启动。

use uuid::Uuid;

use crate::backend::{BackendCommand, PtyRequest};
use crate::model::{HostId, SessionId, SessionStatus, WorkspacePage};
use crate::terminal::TerminalTabState;

use super::super::{AppState, AppUpdateOutcome};
use super::{connect_command_with_known_hosts, missing_host, queued_outcome};

impl AppState {
    /// 打开交互式 Shell，并把连接和 PTY 请求排入后端队列。
    pub(in crate::model::app_state) fn open_shell(&mut self, host_id: HostId) -> AppUpdateOutcome {
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
        let known_hosts = self.storage.known_hosts.clone();
        self.backend_commands.extend([
            connect_command_with_known_hosts(session_id, &host, known_hosts),
            BackendCommand::OpenShell { session_id, pty },
        ]);

        queued_outcome(2)
    }

    /// 从最近连接记录重新打开交互式 Shell。
    pub(in crate::model::app_state) fn open_recent_connection(
        &mut self,
        host_id: HostId,
    ) -> AppUpdateOutcome {
        self.open_shell(host_id)
    }
}
