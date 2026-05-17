//! 会话标签页生命周期处理。
//!
//! 负责关闭、激活会话标签页，以及清理关联的终端、SFTP 和隧道运行态。

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{SessionId, SessionKind, SessionStatus, SessionTab, TransferId, TunnelStatus};

use super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn close_session_tab(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        let Some(tab) = self
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到会话标签页：{}", session_id.0)),
                ..AppUpdateOutcome::default()
            };
        };

        let can_cancel_pending_tunnel_launch = match &tab.kind {
            SessionKind::Tunnel { rule_name } => {
                self.can_cancel_pending_tunnel_launch(session_id, rule_name)
            }
            _ => false,
        };

        if let SessionKind::Tunnel { rule_name } = &tab.kind {
            if self.tunnel_requires_stop_before_close(rule_name)
                && !can_cancel_pending_tunnel_launch
            {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 仍在运行，请先停止再关闭标签页")),
                    ..AppUpdateOutcome::default()
                };
            }
        }

        let pending_cleanup = self.remove_pending_backend_commands_for_session(session_id);
        let command_history_finished = self.finish_remote_command_history_for_closed_tab(&tab);
        let should_disconnect = should_disconnect_on_close(&tab, &pending_cleanup);
        let session_closed = self.sessions.close_tab(session_id);
        let terminal_closed = self.terminal.close_tab(session_id);
        let sftp_browser_removed = self.remove_sftp_browser_after_tab_close(&tab);
        let tunnel_runtime_removed = self.remove_tunnel_runtime_after_tab_close(&tab);

        if should_disconnect {
            self.backend_commands
                .push(BackendCommand::Disconnect { session_id });
        }

        AppUpdateOutcome {
            state_changed: session_closed
                || terminal_closed
                || sftp_browser_removed
                || tunnel_runtime_removed
                || pending_cleanup.changed()
                || command_history_finished,
            queued_backend_commands: usize::from(should_disconnect),
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn activate_session_tab(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        if !self.sessions.tabs.iter().any(|tab| tab.id == session_id) {
            return AppUpdateOutcome {
                error: Some(format!("找不到会话标签页：{}", session_id.0)),
                ..AppUpdateOutcome::default()
            };
        }

        let terminal_changed = self.terminal.set_active_tab(session_id);
        let session_changed = self.sessions.active_tab != Some(session_id);
        self.sessions.active_tab = Some(session_id);

        AppUpdateOutcome {
            state_changed: terminal_changed || session_changed,
            ..AppUpdateOutcome::default()
        }
    }

    fn tunnel_requires_stop_before_close(&self, rule_name: &str) -> bool {
        self.sessions.tunnels.iter().any(|tunnel| {
            tunnel.rule_name == rule_name
                && matches!(
                    tunnel.status,
                    TunnelStatus::Starting | TunnelStatus::Running | TunnelStatus::Stopping
                )
        })
    }

    fn can_cancel_pending_tunnel_launch(&self, session_id: SessionId, rule_name: &str) -> bool {
        let runtime_is_starting = self.sessions.tunnels.iter().any(|tunnel| {
            tunnel.rule_name == rule_name && matches!(tunnel.status, TunnelStatus::Starting)
        });

        runtime_is_starting
            && self
                .backend_commands
                .iter()
                .any(|command| is_tunnel_launch_command(command, session_id, rule_name))
    }

    fn remove_sftp_browser_after_tab_close(&mut self, tab: &SessionTab) -> bool {
        if !matches!(tab.kind, SessionKind::Sftp) {
            return false;
        }

        let Some(host_id) = tab.host_id else {
            return false;
        };
        let has_other_sftp_tab =
            self.sessions.tabs.iter().any(|other| {
                other.host_id == Some(host_id) && matches!(other.kind, SessionKind::Sftp)
            });

        if has_other_sftp_tab {
            return self.reassign_sftp_browser_after_tab_close(tab);
        }

        let before = self.sessions.sftp_browsers.len();
        self.sessions
            .sftp_browsers
            .retain(|browser| browser.host_id != host_id);
        before != self.sessions.sftp_browsers.len()
    }

    fn reassign_sftp_browser_after_tab_close(&mut self, tab: &SessionTab) -> bool {
        let Some(host_id) = tab.host_id else {
            return false;
        };

        let Some(browser) = self
            .sessions
            .sftp_browsers
            .iter_mut()
            .find(|browser| browser.host_id == host_id && browser.session_id == tab.id)
        else {
            return false;
        };

        let Some(next_session_id) = self
            .sessions
            .tabs
            .iter()
            .rev()
            .find(|other| {
                other.id != tab.id
                    && other.host_id == Some(host_id)
                    && matches!(other.kind, SessionKind::Sftp)
            })
            .map(|other| other.id)
        else {
            return false;
        };

        browser.session_id = next_session_id;
        true
    }

    fn remove_tunnel_runtime_after_tab_close(&mut self, tab: &SessionTab) -> bool {
        let SessionKind::Tunnel { rule_name } = &tab.kind else {
            return false;
        };

        let before = self.sessions.tunnels.len();
        self.sessions
            .tunnels
            .retain(|tunnel| tunnel.rule_name != *rule_name);
        before != self.sessions.tunnels.len()
    }

    fn remove_pending_backend_commands_for_session(
        &mut self,
        session_id: SessionId,
    ) -> PendingCloseCommandCleanup {
        let mut removed_connect = false;
        let mut removed_start_tunnel = false;
        let mut transfer_ids = Vec::new();
        let removed_count = self.backend_commands.retain(|command| {
            if command.session_id() != session_id {
                return true;
            }

            removed_connect |= matches!(command, BackendCommand::Connect { .. });
            removed_start_tunnel |= matches!(command, BackendCommand::StartTunnel { .. });
            if let Some(transfer_id) = sftp_transfer_id(command) {
                transfer_ids.push(transfer_id);
            }
            false
        });
        let cancelled_transfer_count = transfer_ids
            .into_iter()
            .filter(|transfer_id| self.sessions.cancel_queued_transfer(*transfer_id))
            .count();

        PendingCloseCommandCleanup {
            removed_count,
            removed_connect,
            removed_start_tunnel,
            cancelled_transfer_count,
        }
    }
}

fn should_disconnect_on_close(tab: &SessionTab, cleanup: &PendingCloseCommandCleanup) -> bool {
    let closed_before_connect = cleanup.removed_connect;
    let cancelled_connected_tunnel_launch = matches!(tab.kind, SessionKind::Tunnel { .. })
        && cleanup.removed_start_tunnel
        && !closed_before_connect;

    (cancelled_connected_tunnel_launch || !matches!(tab.kind, SessionKind::Tunnel { .. }))
        && !closed_before_connect
        && !matches!(
            tab.status,
            SessionStatus::Disconnected | SessionStatus::Failed { .. }
        )
}

fn sftp_transfer_id(command: &BackendCommand) -> Option<TransferId> {
    let BackendCommand::Sftp { request, .. } = command else {
        return None;
    };

    match request {
        SftpRequest::Upload { id, .. } | SftpRequest::Download { id, .. } => Some(*id),
        _ => None,
    }
}

fn is_tunnel_launch_command(
    command: &BackendCommand,
    session_id: SessionId,
    rule_name: &str,
) -> bool {
    match command {
        BackendCommand::Connect {
            session_id: command_session_id,
            ..
        } => *command_session_id == session_id,
        BackendCommand::StartTunnel {
            session_id: command_session_id,
            request,
        } => *command_session_id == session_id && request.rule.name == rule_name,
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PendingCloseCommandCleanup {
    removed_count: usize,
    removed_connect: bool,
    removed_start_tunnel: bool,
    cancelled_transfer_count: usize,
}

impl PendingCloseCommandCleanup {
    fn changed(self) -> bool {
        self.removed_count > 0 || self.cancelled_transfer_count > 0
    }
}
