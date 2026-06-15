//! SFTP 浏览器的可用会话选择和错误构造。

use crate::core::CoreState;
use crate::model::{HostId, SessionId, SessionKind, SessionStatus};

use super::super::AppUpdateOutcome;

impl CoreState {
    pub(in crate::model::app_state) fn claimed_current_sftp_dir_for_host(
        &mut self,
        host_id: HostId,
    ) -> SftpCurrentDirLookup {
        if self.current_sftp_dir_for_host(host_id).is_none() {
            return SftpCurrentDirLookup::MissingBrowser;
        }

        if self.claim_sftp_session_id_for_host(host_id).is_none() {
            return SftpCurrentDirLookup::MissingActiveSession;
        }

        self.current_sftp_dir_for_host(host_id)
            .map(SftpCurrentDirLookup::Found)
            .unwrap_or(SftpCurrentDirLookup::MissingBrowser)
    }

    pub(in crate::model::app_state) fn sftp_session_id_for_host(
        &self,
        host_id: HostId,
    ) -> Option<SessionId> {
        self.sftp_browser_owner_session_id(host_id)
            .filter(|session_id| self.sftp_session_can_accept_commands(*session_id, host_id))
            .or_else(|| self.fallback_sftp_session_id_for_host(host_id))
    }

    pub(in crate::model::app_state) fn claim_sftp_session_id_for_host(
        &mut self,
        host_id: HostId,
    ) -> Option<SessionId> {
        let session_id = self.sftp_session_id_for_host(host_id)?;
        self.sessions
            .reassign_sftp_browser_session(host_id, session_id);
        Some(session_id)
    }

    fn sftp_browser_owner_session_id(&self, host_id: HostId) -> Option<SessionId> {
        self.sessions
            .sftp_browsers
            .iter()
            .find(|browser| browser.host_id == host_id)
            .map(|browser| browser.session_id)
    }

    fn fallback_sftp_session_id_for_host(&self, host_id: HostId) -> Option<SessionId> {
        self.sessions
            .tabs
            .iter()
            .rev()
            .find(|tab| {
                tab.host_id == Some(host_id)
                    && matches!(tab.kind, SessionKind::Sftp)
                    && sftp_tab_can_accept_commands(&tab.status)
            })
            .map(|tab| tab.id)
    }

    fn sftp_session_can_accept_commands(&self, session_id: SessionId, host_id: HostId) -> bool {
        self.sessions.tabs.iter().any(|tab| {
            tab.id == session_id
                && tab.host_id == Some(host_id)
                && matches!(tab.kind, SessionKind::Sftp)
                && sftp_tab_can_accept_commands(&tab.status)
        })
    }
}

pub(in crate::model::app_state) enum SftpCurrentDirLookup {
    Found(String),
    MissingBrowser,
    MissingActiveSession,
}

pub(in crate::model::app_state) fn missing_sftp_browser(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到该主机的 SFTP 浏览器：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

pub(in crate::model::app_state) fn missing_active_sftp_session(
    host_id: HostId,
) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("该主机没有可用的 SFTP 会话：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn sftp_tab_can_accept_commands(status: &SessionStatus) -> bool {
    !status.is_terminal()
}
