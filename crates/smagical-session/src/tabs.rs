//! 会话标签页的打开、状态更新和关闭操作。

use smagical_core::{
    CommandHistoryId, HostId, SessionId, SessionKind, SessionStatus, SessionTab, TunnelRule,
    WorkspaceTabSnapshot,
};

use super::SessionManager;

impl SessionManager {
    /// 打开一个本地 Shell 标签页。
    pub fn open_local_shell_tab(&mut self, id: SessionId, title: impl Into<String>) {
        self.push_tab(SessionTab {
            id,
            host_id: None,
            kind: SessionKind::LocalShell,
            title: title.into(),
            status: SessionStatus::Connected,
        });
    }

    /// 打开一个新的交互式 Shell 标签页。
    pub fn open_shell_tab(&mut self, id: SessionId, host_id: HostId, title: impl Into<String>) {
        self.push_tab(SessionTab {
            id,
            host_id: Some(host_id),
            kind: SessionKind::Shell,
            title: title.into(),
            status: SessionStatus::Created,
        });
    }

    /// 打开一个远程命令执行标签页。
    pub fn open_remote_command_tab(
        &mut self,
        id: SessionId,
        host_id: HostId,
        command: impl Into<String>,
        history_id: Option<CommandHistoryId>,
    ) {
        let command = command.into();

        self.push_tab(SessionTab {
            id,
            host_id: Some(host_id),
            kind: SessionKind::RemoteCommand {
                command: command.clone(),
                history_id,
            },
            title: command,
            status: SessionStatus::Created,
        });
    }

    /// 打开一个隧道管理标签页。
    pub fn open_tunnel_tab(&mut self, id: SessionId, host_id: HostId, rule: &TunnelRule) {
        self.push_tab(SessionTab {
            id,
            host_id: Some(host_id),
            kind: SessionKind::Tunnel {
                rule_name: rule.name.clone(),
            },
            title: rule.display_endpoint(),
            status: SessionStatus::Created,
        });
    }

    /// 更新标签页状态。
    pub fn set_status(&mut self, id: SessionId, status: SessionStatus) -> bool {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
            if tab.status.is_terminal() {
                return false;
            }

            let is_terminal = status.is_terminal();
            tab.status = status;
            self.sync_active_index_for_status(id, is_terminal);
            true
        } else {
            false
        }
    }

    /// 关闭标签页，并同步活动连接索引。
    pub fn close_tab(&mut self, id: SessionId) -> bool {
        let before = self.tabs.len();
        self.tabs.retain(|tab| tab.id != id);
        self.active.retain(|active_id| *active_id != id);

        if self.active_tab == Some(id) {
            self.active_tab = self.tabs.last().map(|tab| tab.id);
        }

        before != self.tabs.len()
    }

    /// 返回需要后台轮询输出的交互式 shell 标签页。
    pub fn interactive_shell_tab_ids(&self) -> Vec<SessionId> {
        self.tabs
            .iter()
            .filter(|tab| self.can_drain_interactive_shell(tab.id))
            .map(|tab| tab.id)
            .collect()
    }

    /// 判断后台输出泵是否仍应抽取该会话的交互式 shell 输出。
    pub fn can_drain_interactive_shell(&self, id: SessionId) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == id)
            .is_some_and(|tab| {
                matches!(tab.kind, SessionKind::LocalShell | SessionKind::Shell)
                    && matches!(tab.status, SessionStatus::Connected)
            })
    }

    /// 判断该会话当前是否仍允许交互式 shell 输入命令发往后端。
    pub fn can_send_interactive_shell_input(&self, id: SessionId) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == id)
            .is_some_and(SessionTab::can_accept_terminal_input)
    }

    /// 判断打开 shell 命令是否仍允许发往后端执行器。
    pub fn can_execute_open_shell_command(&self, id: SessionId) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == id)
            .is_some_and(|tab| matches!(tab.kind, SessionKind::Shell) && !tab.status.is_terminal())
    }

    /// 将交互式 shell 标签页标记为已打开，忽略其他类型或终态标签页的串台事件。
    pub fn mark_shell_opened(&mut self, id: SessionId) -> bool {
        if !self.can_execute_open_shell_command(id) {
            return false;
        }

        self.set_status(id, SessionStatus::Connected)
    }

    /// 判断远程命令执行请求是否仍允许发往后端执行器。
    pub fn can_execute_remote_command(&self, id: SessionId) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == id)
            .is_some_and(|tab| {
                matches!(tab.kind, SessionKind::RemoteCommand { .. }) && !tab.status.is_terminal()
            })
    }

    /// 将远程命令标签页标记为运行中，忽略其他类型或终态标签页的串台事件。
    pub fn mark_remote_command_started(&mut self, id: SessionId) -> bool {
        if !self.can_execute_remote_command(id) {
            return false;
        }

        self.set_status(id, SessionStatus::RunningCommand)
    }

    /// 将会产生进程退出事件的标签页标记为完成，忽略其他类型或终态标签页的串台事件。
    pub fn mark_process_exited(&mut self, id: SessionId, exit_code: Option<i32>) -> bool {
        let Some(tab) = self.tabs.iter().find(|tab| tab.id == id) else {
            return false;
        };

        if !matches!(
            tab.kind,
            SessionKind::Shell | SessionKind::RemoteCommand { .. }
        ) || tab.status.is_terminal()
        {
            return false;
        }

        let status = match exit_code {
            Some(0) | None => SessionStatus::Disconnected,
            Some(code) => SessionStatus::Failed {
                reason: format!("remote command exited with {code}"),
            },
        };

        self.set_status(id, status)
    }

    /// 将远程连接标签页标记为正在连接，忽略本地 shell、缺失或终态标签页。
    pub fn mark_remote_connecting(&mut self, id: SessionId) -> bool {
        if !self.can_update_remote_connection_status(id) {
            return false;
        }

        self.set_status(id, SessionStatus::Connecting)
    }

    /// 用户主动重连远程 Shell 时，允许终态标签页重新进入连接流程。
    pub fn mark_shell_reconnecting(&mut self, id: SessionId) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
            return false;
        };

        if !matches!(tab.kind, SessionKind::Shell)
            || !matches!(
                tab.status,
                SessionStatus::Disconnected | SessionStatus::Failed { .. }
            )
        {
            return false;
        }

        tab.status = SessionStatus::Reconnecting;
        self.sync_active_index_for_status(id, false);
        true
    }

    /// 将远程连接标签页标记为认证中，忽略本地 shell、缺失或终态标签页。
    pub fn mark_remote_authenticating(&mut self, id: SessionId) -> bool {
        if !self.can_update_remote_connection_status(id) {
            return false;
        }

        self.set_status(id, SessionStatus::Authenticating)
    }

    /// 将远程连接标签页标记为认证完成，忽略本地 shell、缺失或终态标签页。
    pub fn mark_remote_authenticated(&mut self, id: SessionId) -> bool {
        if !self.can_update_remote_connection_status(id) {
            return false;
        }

        self.set_status(id, SessionStatus::Connected)
    }

    /// 将后端已连接事件写入标签页，允许本地 shell 和远程连接标签页。
    pub fn mark_backend_connected(&mut self, id: SessionId) -> bool {
        let Some(tab) = self.tabs.iter().find(|tab| tab.id == id) else {
            return false;
        };

        if !matches!(
            tab.kind,
            SessionKind::LocalShell
                | SessionKind::Shell
                | SessionKind::RemoteCommand { .. }
                | SessionKind::Sftp
                | SessionKind::Tunnel { .. }
        ) || tab.status.is_terminal()
        {
            return false;
        }

        self.set_status(id, SessionStatus::Connected)
    }

    /// 标记会话失败，并集中收敛 SFTP、传输和隧道运行态。
    pub fn mark_session_failed(&mut self, id: SessionId, reason: impl Into<String>) -> bool {
        let reason = reason.into();
        let session_updated = self.set_status(
            id,
            SessionStatus::Failed {
                reason: reason.clone(),
            },
        );
        let sftp_updated = self.fail_sftp_runtime_for_session(id, reason.clone());
        let tunnel_updated = self.fail_tunnel_for_session(id, reason);

        session_updated || sftp_updated || tunnel_updated
    }

    /// 标记会话断开，并集中收敛 SFTP、传输和隧道运行态。
    pub fn mark_session_disconnected(&mut self, id: SessionId) -> bool {
        let session_updated = self.set_status(id, SessionStatus::Disconnected);
        let sftp_updated = self.fail_sftp_runtime_for_session(id, "SFTP 会话已断开");
        let tunnel_updated = self.stop_tunnel_for_session(id);

        session_updated || sftp_updated || tunnel_updated
    }

    /// 判断连接命令是否仍允许发往后端执行器。
    pub fn can_execute_connect_command(&self, id: SessionId, host_id: HostId) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == id)
            .is_some_and(|tab| {
                tab.host_id == Some(host_id)
                    && matches!(
                        tab.kind,
                        SessionKind::Shell
                            | SessionKind::RemoteCommand { .. }
                            | SessionKind::Sftp
                            | SessionKind::Tunnel { .. }
                    )
                    && !tab.status.is_terminal()
            })
    }

    /// 判断终端缓冲是否仍可被对应会话更新。
    pub fn can_update_terminal_buffer(&self, id: SessionId) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == id)
            .is_some_and(|tab| !tab.status.is_terminal())
    }

    /// 从工作区快照恢复可见标签页元数据，不自动建立网络连接。
    pub fn restore_tabs_from_workspace(
        &mut self,
        tabs: &[WorkspaceTabSnapshot],
        active_tab: Option<SessionId>,
    ) {
        self.tabs = tabs
            .iter()
            .map(|snapshot| SessionTab {
                id: snapshot.session_id,
                host_id: snapshot.host_id,
                kind: snapshot.kind.clone(),
                title: snapshot.title.clone(),
                status: SessionStatus::Disconnected,
            })
            .collect();
        self.active.clear();
        self.active_tab = active_tab
            .filter(|active_id| self.tabs.iter().any(|tab| tab.id == *active_id))
            .or_else(|| self.tabs.last().map(|tab| tab.id));
    }

    fn sync_active_index_for_status(&mut self, id: SessionId, is_terminal: bool) {
        if is_terminal {
            self.active.retain(|active_id| *active_id != id);
        } else if !self.active.contains(&id) {
            self.active.push(id);
        }
    }

    fn can_update_remote_connection_status(&self, id: SessionId) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == id)
            .is_some_and(|tab| {
                tab.host_id.is_some()
                    && matches!(
                        tab.kind,
                        SessionKind::Shell
                            | SessionKind::RemoteCommand { .. }
                            | SessionKind::Sftp
                            | SessionKind::Tunnel { .. }
                    )
                    && !tab.status.is_terminal()
            })
    }

    fn fail_sftp_runtime_for_session(&mut self, id: SessionId, reason: impl Into<String>) -> bool {
        let reason = reason.into();
        let browser_updated = self.reassign_sftp_browser_after_session_loss(id)
            || self.fail_sftp_browser_for_session(id, reason.clone());
        let transfers_updated = self.fail_transfers_for_session(id, reason);

        browser_updated || transfers_updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::{
        DEFAULT_LOCAL_TERMINAL_TITLE, TransferDirection, TransferId, TransferStatus, TransferTask,
        TunnelKind, TunnelStatus,
    };
    use uuid::Uuid;

    fn host_id() -> HostId {
        HostId(Uuid::new_v4())
    }

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    fn tunnel_rule(name: &str) -> TunnelRule {
        TunnelRule {
            name: name.to_owned(),
            kind: TunnelKind::Local,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 15432,
            target_host: "10.0.0.5".to_owned(),
            target_port: 5432,
            auto_start: false,
        }
    }

    fn transfer_task(id: TransferId, session_id: SessionId, host_id: HostId) -> TransferTask {
        TransferTask {
            id,
            session_id,
            host_id,
            direction: TransferDirection::Download,
            local_path: "C:/tmp/syslog".to_owned(),
            remote_path: "/var/log/syslog".to_owned(),
            total_bytes: Some(100),
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        }
    }

    #[test]
    fn opening_shell_tab_tracks_active_session_and_tab() {
        let mut sessions = SessionManager::default();
        let id = session_id();
        let host_id = host_id();

        sessions.open_shell_tab(id, host_id, "production");

        assert_eq!(sessions.active_count(), 1);
        assert_eq!(sessions.tab_count(), 1);
        assert_eq!(sessions.active_tab, Some(id));
        assert_eq!(sessions.tabs[0].host_id, Some(host_id));
        assert!(matches!(sessions.tabs[0].kind, SessionKind::Shell));
        assert!(matches!(sessions.tabs[0].status, SessionStatus::Created));
    }

    #[test]
    fn opening_local_shell_tab_tracks_active_session_without_host() {
        let mut sessions = SessionManager::default();
        let id = session_id();

        sessions.open_local_shell_tab(id, DEFAULT_LOCAL_TERMINAL_TITLE);

        assert_eq!(sessions.tab_count(), 1);
        assert_eq!(sessions.active_tab, Some(id));
        assert_eq!(sessions.tabs[0].host_id, None);
        assert!(matches!(sessions.tabs[0].kind, SessionKind::LocalShell));
        assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
    }

    #[test]
    fn opening_remote_command_tab_sets_command_title_and_kind() {
        let mut sessions = SessionManager::default();
        let id = session_id();

        sessions.open_remote_command_tab(id, host_id(), "uptime", None);

        assert_eq!(sessions.tabs[0].title, "uptime");
        assert!(matches!(
            &sessions.tabs[0].kind,
            SessionKind::RemoteCommand { command, history_id }
                if command == "uptime" && history_id.is_none()
        ));
    }

    #[test]
    fn opening_tunnel_tab_uses_rule_name_and_endpoint_title() {
        let mut sessions = SessionManager::default();
        let id = session_id();
        let rule = tunnel_rule("local-db");

        sessions.open_tunnel_tab(id, host_id(), &rule);

        assert_eq!(sessions.tab_count(), 1);
        assert_eq!(sessions.tabs[0].title, "L 127.0.0.1:15432 -> 10.0.0.5:5432");
        assert!(matches!(
            &sessions.tabs[0].kind,
            SessionKind::Tunnel { rule_name } if rule_name == "local-db"
        ));
    }

    #[test]
    fn set_status_updates_existing_tab_only() {
        let mut sessions = SessionManager::default();
        let id = session_id();

        sessions.open_shell_tab(id, host_id(), "production");

        assert!(sessions.set_status(id, SessionStatus::Connected));
        assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
        assert!(!sessions.set_status(
            session_id(),
            SessionStatus::Failed {
                reason: "missing".to_owned()
            }
        ));
    }

    #[test]
    fn terminal_status_removes_session_from_active_index() {
        let mut sessions = SessionManager::default();
        let first_id = session_id();
        let second_id = session_id();

        sessions.open_shell_tab(first_id, host_id(), "first");
        sessions.open_shell_tab(second_id, host_id(), "second");

        assert!(sessions.set_status(first_id, SessionStatus::Disconnected));
        assert_eq!(sessions.active, vec![second_id]);

        assert!(sessions.set_status(
            second_id,
            SessionStatus::Failed {
                reason: "network".to_owned()
            }
        ));
        assert!(sessions.active.is_empty());
    }

    #[test]
    fn terminal_session_status_ignores_late_non_terminal_status() {
        let mut sessions = SessionManager::default();
        let disconnected_id = session_id();
        let failed_id = session_id();

        sessions.open_shell_tab(disconnected_id, host_id(), "disconnected");
        sessions.open_shell_tab(failed_id, host_id(), "failed");
        assert!(sessions.set_status(disconnected_id, SessionStatus::Disconnected));
        assert!(sessions.set_status(
            failed_id,
            SessionStatus::Failed {
                reason: "network".to_owned()
            }
        ));

        assert!(!sessions.set_status(disconnected_id, SessionStatus::Connected));
        assert!(!sessions.set_status(failed_id, SessionStatus::Connecting));

        assert!(sessions.active.is_empty());
        assert!(matches!(
            sessions.tabs[0].status,
            SessionStatus::Disconnected
        ));
        assert!(matches!(
            &sessions.tabs[1].status,
            SessionStatus::Failed { reason } if reason == "network"
        ));
    }

    #[test]
    fn terminal_buffer_update_acceptance_depends_on_session_status() {
        let mut sessions = SessionManager::default();
        let shell_id = session_id();
        let failed_id = session_id();

        sessions.open_shell_tab(shell_id, host_id(), "shell");
        sessions.open_shell_tab(failed_id, host_id(), "failed");

        assert!(sessions.can_update_terminal_buffer(shell_id));
        assert!(!sessions.can_update_terminal_buffer(session_id()));

        assert!(sessions.set_status(shell_id, SessionStatus::Disconnected));
        assert!(sessions.set_status(
            failed_id,
            SessionStatus::Failed {
                reason: "network".to_owned()
            }
        ));

        assert!(!sessions.can_update_terminal_buffer(shell_id));
        assert!(!sessions.can_update_terminal_buffer(failed_id));
    }

    #[test]
    fn close_tab_removes_active_session_and_selects_previous_tab() {
        let mut sessions = SessionManager::default();
        let first_id = session_id();
        let second_id = session_id();

        sessions.open_shell_tab(first_id, host_id(), "first");
        sessions.open_shell_tab(second_id, host_id(), "second");

        assert!(sessions.close_tab(second_id));
        assert_eq!(sessions.active_count(), 1);
        assert_eq!(sessions.tab_count(), 1);
        assert_eq!(sessions.active_tab, Some(first_id));
        assert!(!sessions.close_tab(second_id));
    }

    #[test]
    fn interactive_shell_tab_ids_include_only_connected_shell_tabs() {
        let mut sessions = SessionManager::default();
        let local_id = session_id();
        let shell_id = session_id();
        let command_id = session_id();

        sessions.open_local_shell_tab(local_id, DEFAULT_LOCAL_TERMINAL_TITLE);
        sessions.open_shell_tab(shell_id, host_id(), "production");
        sessions.open_remote_command_tab(command_id, host_id(), "uptime", None);
        sessions.set_status(shell_id, SessionStatus::Connected);

        assert_eq!(
            sessions.interactive_shell_tab_ids(),
            vec![local_id, shell_id]
        );
        assert!(sessions.can_drain_interactive_shell(local_id));
        assert!(sessions.can_drain_interactive_shell(shell_id));
        assert!(!sessions.can_drain_interactive_shell(command_id));
        assert!(sessions.can_send_interactive_shell_input(local_id));
        assert!(sessions.can_send_interactive_shell_input(shell_id));
        assert!(!sessions.can_send_interactive_shell_input(command_id));
    }

    #[test]
    fn shell_open_command_acceptance_requires_non_terminal_shell_tab() {
        let mut sessions = SessionManager::default();
        let shell_id = session_id();
        let command_id = session_id();
        let local_id = session_id();

        sessions.open_shell_tab(shell_id, host_id(), "production");
        sessions.open_remote_command_tab(command_id, host_id(), "uptime", None);
        sessions.open_local_shell_tab(local_id, DEFAULT_LOCAL_TERMINAL_TITLE);

        assert!(sessions.can_execute_open_shell_command(shell_id));
        assert!(!sessions.can_execute_open_shell_command(command_id));
        assert!(!sessions.can_execute_open_shell_command(local_id));
        assert!(!sessions.can_execute_open_shell_command(session_id()));

        assert!(sessions.set_status(shell_id, SessionStatus::Disconnected));

        assert!(!sessions.can_execute_open_shell_command(shell_id));
    }

    #[test]
    fn shell_opened_status_requires_non_terminal_shell_tab() {
        let mut sessions = SessionManager::default();
        let shell_id = session_id();
        let command_id = session_id();
        let host_id = host_id();

        sessions.open_shell_tab(shell_id, host_id, "production");
        sessions.open_remote_command_tab(command_id, host_id, "uptime", None);

        assert!(sessions.mark_shell_opened(shell_id));
        assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));

        assert!(!sessions.mark_shell_opened(command_id));
        assert!(!matches!(sessions.tabs[1].status, SessionStatus::Connected));

        assert!(sessions.set_status(shell_id, SessionStatus::Disconnected));
        assert!(!sessions.mark_shell_opened(shell_id));
    }

    #[test]
    fn remote_command_execution_acceptance_requires_non_terminal_command_tab() {
        let mut sessions = SessionManager::default();
        let shell_id = session_id();
        let command_id = session_id();

        sessions.open_shell_tab(shell_id, host_id(), "production");
        sessions.open_remote_command_tab(command_id, host_id(), "uptime", None);

        assert!(sessions.can_execute_remote_command(command_id));
        assert!(!sessions.can_execute_remote_command(shell_id));
        assert!(!sessions.can_execute_remote_command(session_id()));

        assert!(sessions.set_status(
            command_id,
            SessionStatus::Failed {
                reason: "network".to_owned(),
            }
        ));

        assert!(!sessions.can_execute_remote_command(command_id));
    }

    #[test]
    fn remote_command_started_status_requires_non_terminal_command_tab() {
        let mut sessions = SessionManager::default();
        let shell_id = session_id();
        let command_id = session_id();
        let host_id = host_id();

        sessions.open_shell_tab(shell_id, host_id, "production");
        sessions.open_remote_command_tab(command_id, host_id, "uptime", None);

        assert!(sessions.mark_remote_command_started(command_id));
        assert!(matches!(
            sessions.tabs[1].status,
            SessionStatus::RunningCommand
        ));

        assert!(!sessions.mark_remote_command_started(shell_id));
        assert!(!matches!(
            sessions.tabs[0].status,
            SessionStatus::RunningCommand
        ));

        assert!(sessions.set_status(command_id, SessionStatus::Disconnected));
        assert!(!sessions.mark_remote_command_started(command_id));
    }

    #[test]
    fn process_exited_status_requires_non_terminal_process_tab() {
        let mut sessions = SessionManager::default();
        let shell_id = session_id();
        let command_id = session_id();
        let sftp_id = session_id();
        let host_id = host_id();

        sessions.open_shell_tab(shell_id, host_id, "production");
        sessions.open_remote_command_tab(command_id, host_id, "uptime", None);
        sessions.open_sftp_tab(sftp_id, host_id, "/home/ops");

        assert!(sessions.mark_process_exited(shell_id, None));
        assert!(matches!(
            sessions.tabs[0].status,
            SessionStatus::Disconnected
        ));

        assert!(sessions.mark_process_exited(command_id, Some(7)));
        assert!(matches!(
            &sessions.tabs[1].status,
            SessionStatus::Failed { reason } if reason == "remote command exited with 7"
        ));

        assert!(!sessions.mark_process_exited(sftp_id, Some(0)));
        assert!(matches!(sessions.tabs[2].status, SessionStatus::Created));

        assert!(!sessions.mark_process_exited(shell_id, Some(0)));
    }

    #[test]
    fn remote_connection_status_requires_non_terminal_remote_tab() {
        let mut sessions = SessionManager::default();
        let shell_id = session_id();
        let sftp_id = session_id();
        let local_id = session_id();
        let host_id = host_id();

        sessions.open_shell_tab(shell_id, host_id, "production");
        sessions.open_sftp_tab(sftp_id, host_id, "/home/ops");
        sessions.open_local_shell_tab(local_id, DEFAULT_LOCAL_TERMINAL_TITLE);

        assert!(sessions.mark_remote_connecting(shell_id));
        assert!(matches!(sessions.tabs[0].status, SessionStatus::Connecting));

        assert!(sessions.mark_remote_authenticating(sftp_id));
        assert!(matches!(
            sessions.tabs[1].status,
            SessionStatus::Authenticating
        ));

        assert!(!sessions.mark_remote_connecting(local_id));
        assert!(matches!(sessions.tabs[2].status, SessionStatus::Connected));

        assert!(sessions.set_status(sftp_id, SessionStatus::Disconnected));
        assert!(!sessions.mark_remote_authenticated(sftp_id));
    }

    #[test]
    fn shell_reconnecting_status_requires_terminal_shell_tab() {
        let mut sessions = SessionManager::default();
        let disconnected_id = session_id();
        let failed_id = session_id();
        let connected_id = session_id();
        let command_id = session_id();
        let host_id = host_id();

        sessions.open_shell_tab(disconnected_id, host_id, "disconnected");
        sessions.open_shell_tab(failed_id, host_id, "failed");
        sessions.open_shell_tab(connected_id, host_id, "connected");
        sessions.open_remote_command_tab(command_id, host_id, "uptime", None);
        assert!(sessions.set_status(disconnected_id, SessionStatus::Disconnected));
        assert!(sessions.set_status(
            failed_id,
            SessionStatus::Failed {
                reason: "network".to_owned()
            }
        ));
        assert!(sessions.set_status(connected_id, SessionStatus::Connected));
        assert!(sessions.set_status(command_id, SessionStatus::Disconnected));

        assert!(sessions.mark_shell_reconnecting(disconnected_id));
        assert!(sessions.mark_shell_reconnecting(failed_id));
        assert!(!sessions.mark_shell_reconnecting(connected_id));
        assert!(!sessions.mark_shell_reconnecting(command_id));

        assert!(matches!(
            sessions.tabs[0].status,
            SessionStatus::Reconnecting
        ));
        assert!(matches!(
            sessions.tabs[1].status,
            SessionStatus::Reconnecting
        ));
        assert!(matches!(sessions.tabs[2].status, SessionStatus::Connected));
        assert!(matches!(
            sessions.tabs[3].status,
            SessionStatus::Disconnected
        ));
    }

    #[test]
    fn backend_connected_status_accepts_local_and_remote_non_terminal_tabs() {
        let mut sessions = SessionManager::default();
        let local_id = session_id();
        let command_id = session_id();
        let host_id = host_id();

        sessions.open_local_shell_tab(local_id, DEFAULT_LOCAL_TERMINAL_TITLE);
        sessions.open_remote_command_tab(command_id, host_id, "uptime", None);

        assert!(sessions.mark_backend_connected(local_id));
        assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));

        assert!(sessions.mark_backend_connected(command_id));
        assert!(matches!(sessions.tabs[1].status, SessionStatus::Connected));

        assert!(sessions.set_status(command_id, SessionStatus::Disconnected));
        assert!(!sessions.mark_backend_connected(command_id));
    }

    #[test]
    fn failed_session_status_collects_sftp_and_tunnel_runtime() {
        let mut sessions = SessionManager::default();
        let sftp_id = session_id();
        let tunnel_id = session_id();
        let host_id = host_id();
        let rule = tunnel_rule("local-db");
        let transfer_id = TransferId(Uuid::new_v4());

        sessions.open_sftp_tab(sftp_id, host_id, "/home/ops");
        sessions.set_sftp_loading(host_id, true);
        sessions.enqueue_transfer(transfer_task(transfer_id, sftp_id, host_id));
        sessions.open_tunnel_tab(tunnel_id, host_id, &rule);
        sessions.start_tunnel(tunnel_id, &rule, Some(host_id), 10);

        assert!(sessions.mark_session_failed(sftp_id, "connection reset"));
        assert!(matches!(
            &sessions.tabs[0].status,
            SessionStatus::Failed { reason } if reason == "connection reset"
        ));
        assert_eq!(
            sessions.sftp_browsers[0].last_error.as_deref(),
            Some("connection reset")
        );
        assert!(matches!(
            &sessions.transfers[0].status,
            TransferStatus::Failed { reason } if reason == "connection reset"
        ));

        assert!(sessions.mark_session_failed(tunnel_id, "bind failed"));
        assert!(matches!(
            &sessions.tabs[1].status,
            SessionStatus::Failed { reason } if reason == "bind failed"
        ));
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Failed));
        assert_eq!(
            sessions.tunnels[0].last_error.as_deref(),
            Some("bind failed")
        );
    }

    #[test]
    fn disconnected_session_status_collects_sftp_and_tunnel_runtime() {
        let mut sessions = SessionManager::default();
        let sftp_id = session_id();
        let fallback_sftp_id = session_id();
        let tunnel_id = session_id();
        let host_id = host_id();
        let rule = tunnel_rule("local-db");
        let transfer_id = TransferId(Uuid::new_v4());

        sessions.open_sftp_tab(fallback_sftp_id, host_id, "/home/ops");
        sessions.set_status(fallback_sftp_id, SessionStatus::Connected);
        sessions.open_sftp_tab(sftp_id, host_id, "/var/log");
        sessions.set_status(sftp_id, SessionStatus::Connected);
        sessions.set_sftp_loading(host_id, true);
        sessions.enqueue_transfer(transfer_task(transfer_id, sftp_id, host_id));
        sessions.open_tunnel_tab(tunnel_id, host_id, &rule);
        sessions.start_tunnel(tunnel_id, &rule, Some(host_id), 10);
        sessions.mark_tunnel_running(tunnel_id, "local-db");

        assert!(sessions.mark_session_disconnected(sftp_id));
        assert!(matches!(
            sessions.tabs[1].status,
            SessionStatus::Disconnected
        ));
        assert_eq!(sessions.sftp_browsers[0].session_id, fallback_sftp_id);
        assert!(!sessions.sftp_browsers[0].loading);
        assert!(sessions.sftp_browsers[0].last_error.is_none());
        assert!(matches!(
            &sessions.transfers[0].status,
            TransferStatus::Failed { reason } if reason == "SFTP 会话已断开"
        ));

        assert!(sessions.mark_session_disconnected(tunnel_id));
        assert!(matches!(
            sessions.tabs[2].status,
            SessionStatus::Disconnected
        ));
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Stopped));
    }

    #[test]
    fn connect_command_acceptance_requires_matching_non_terminal_remote_tab() {
        let mut sessions = SessionManager::default();
        let shell_id = session_id();
        let command_id = session_id();
        let local_id = session_id();
        let remote_host_id = host_id();
        let other_host_id = host_id();

        sessions.open_shell_tab(shell_id, remote_host_id, "production");
        sessions.open_remote_command_tab(command_id, remote_host_id, "uptime", None);
        sessions.open_local_shell_tab(local_id, DEFAULT_LOCAL_TERMINAL_TITLE);

        assert!(sessions.can_execute_connect_command(shell_id, remote_host_id));
        assert!(sessions.can_execute_connect_command(command_id, remote_host_id));
        assert!(!sessions.can_execute_connect_command(local_id, remote_host_id));
        assert!(!sessions.can_execute_connect_command(shell_id, other_host_id));
        assert!(!sessions.can_execute_connect_command(session_id(), remote_host_id));

        assert!(sessions.set_status(shell_id, SessionStatus::Disconnected));
        assert!(sessions.set_status(
            command_id,
            SessionStatus::Failed {
                reason: "network".to_owned(),
            }
        ));

        assert!(!sessions.can_execute_connect_command(shell_id, remote_host_id));
        assert!(!sessions.can_execute_connect_command(command_id, remote_host_id));
    }

    #[test]
    fn restore_tabs_from_workspace_marks_tabs_disconnected() {
        let mut sessions = SessionManager::default();
        let first_id = session_id();
        let second_id = session_id();

        sessions.restore_tabs_from_workspace(
            &[
                WorkspaceTabSnapshot {
                    session_id: first_id,
                    host_id: Some(host_id()),
                    kind: SessionKind::Shell,
                    title: "first".to_owned(),
                    working_directory: None,
                },
                WorkspaceTabSnapshot {
                    session_id: second_id,
                    host_id: Some(host_id()),
                    kind: SessionKind::RemoteCommand {
                        command: "uptime".to_owned(),
                        history_id: None,
                    },
                    title: "uptime".to_owned(),
                    working_directory: None,
                },
            ],
            Some(first_id),
        );

        assert_eq!(sessions.tab_count(), 2);
        assert_eq!(sessions.active_count(), 0);
        assert_eq!(sessions.active_tab, Some(first_id));
        assert!(matches!(
            sessions.tabs[0].status,
            SessionStatus::Disconnected
        ));
    }
}
