//! 交互式 Shell 启动。

use uuid::Uuid;

use crate::backend::{BackendCommand, PtyRequest};
use crate::model::{HostId, SessionId, SessionKind, SessionStatus, WorkspacePage};
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

    /// 复用已存在的远程 Shell 标签页重新建立连接。
    pub(in crate::model::app_state) fn reconnect_shell(
        &mut self,
        session_id: SessionId,
    ) -> AppUpdateOutcome {
        let Some(tab) = self
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到会话：{}", session_id.0)),
                ..AppUpdateOutcome::default()
            };
        };

        if !matches!(tab.kind, SessionKind::Shell) {
            return AppUpdateOutcome {
                error: Some("只有远程 Shell 标签页支持重新连接".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(host_id) = tab.host_id else {
            return AppUpdateOutcome {
                error: Some("Shell 会话缺少主机标识".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };

        let pty = self
            .terminal
            .tabs
            .iter()
            .find(|terminal_tab| terminal_tab.session_id == session_id)
            .map(|terminal_tab| PtyRequest::xterm(terminal_tab.size))
            .unwrap_or_else(|| PtyRequest::xterm(crate::terminal::TerminalSize::default()));

        if !self.sessions.mark_shell_reconnecting(session_id) {
            return AppUpdateOutcome {
                error: Some("只有已断开或失败的远程 Shell 标签页可以重新连接".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        self.terminal.set_active_tab(session_id);
        self.ui.workspace.active_page = WorkspacePage::Terminal;
        self.record_recent_connection(&host);
        let known_hosts = self.storage.known_hosts.clone();
        self.backend_commands.extend([
            connect_command_with_known_hosts(session_id, &host, known_hosts),
            BackendCommand::OpenShell { session_id, pty },
        ]);

        queued_outcome(2)
    }
}
