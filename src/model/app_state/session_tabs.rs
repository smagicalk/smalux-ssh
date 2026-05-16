//! 会话标签页生命周期处理。
//!
//! 负责关闭、激活会话标签页，以及清理关联的终端、SFTP 和隧道运行态。

use crate::backend::BackendCommand;
use crate::model::{SessionId, SessionKind, SessionStatus, SessionTab, TunnelStatus};

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

        if let SessionKind::Tunnel { rule_name } = &tab.kind {
            if self.tunnel_requires_stop_before_close(rule_name) {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 仍在运行，请先停止再关闭标签页")),
                    ..AppUpdateOutcome::default()
                };
            }
        }

        let should_disconnect = should_disconnect_on_close(&tab);
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
                || tunnel_runtime_removed,
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
            return false;
        }

        let before = self.sessions.sftp_browsers.len();
        self.sessions
            .sftp_browsers
            .retain(|browser| browser.host_id != host_id);
        before != self.sessions.sftp_browsers.len()
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
}

fn should_disconnect_on_close(tab: &SessionTab) -> bool {
    !matches!(tab.kind, SessionKind::Tunnel { .. })
        && !matches!(
            tab.status,
            SessionStatus::Disconnected | SessionStatus::Failed { .. }
        )
}
