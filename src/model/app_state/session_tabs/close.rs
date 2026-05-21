//! 会话标签页关闭流程。

use crate::backend::BackendCommand;
use crate::model::{SessionId, SessionKind};

use super::super::{AppState, AppUpdateOutcome};
use super::pending::should_disconnect_on_close;

impl AppState {
    pub(in crate::model::app_state) fn close_session_tab(
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
            if self.tunnel_requires_stop_before_close(session_id, rule_name)
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
}
