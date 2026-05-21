//! 会话标签页激活流程。

use crate::model::{SessionId, SessionKind, SessionStatus, SessionTab};

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(in crate::model::app_state) fn activate_session_tab(
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

        let terminal_changed = self.terminal.set_active_tab(session_id);
        let session_changed = self.sessions.active_tab != Some(session_id);
        self.sessions.active_tab = Some(session_id);
        let sftp_owner_changed = self.reassign_sftp_browser_on_tab_activation(&tab);

        AppUpdateOutcome {
            state_changed: terminal_changed || session_changed || sftp_owner_changed,
            ..AppUpdateOutcome::default()
        }
    }

    fn reassign_sftp_browser_on_tab_activation(&mut self, tab: &SessionTab) -> bool {
        if !matches!(tab.kind, SessionKind::Sftp) || !sftp_tab_can_accept_browser_owner(&tab.status)
        {
            return false;
        }

        let Some(host_id) = tab.host_id else {
            return false;
        };

        self.sessions.reassign_sftp_browser_session(host_id, tab.id)
    }
}

pub(super) fn sftp_tab_can_accept_browser_owner(status: &SessionStatus) -> bool {
    !status.is_terminal()
}
