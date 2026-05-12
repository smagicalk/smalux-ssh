//! SFTP 浏览器运行态操作。

use crate::model::{
    HostId, SessionId, SessionKind, SessionStatus, SessionTab, SftpBrowserState, SftpEntry,
};

use super::SessionManager;

impl SessionManager {
    /// 打开一个 SFTP 标签页，并初始化浏览目录。
    pub fn open_sftp_tab(
        &mut self,
        id: SessionId,
        host_id: HostId,
        initial_dir: impl Into<String>,
    ) {
        let initial_dir = initial_dir.into();

        self.push_tab(SessionTab {
            id,
            host_id: Some(host_id),
            kind: SessionKind::Sftp,
            title: format!("SFTP {initial_dir}"),
            status: SessionStatus::Created,
        });
        self.upsert_sftp_browser(SftpBrowserState {
            host_id,
            current_dir: initial_dir,
            entries: Vec::new(),
            selected_path: None,
            loading: false,
            last_error: None,
        });
    }

    /// 更新 SFTP 当前目录和目录项。
    pub fn set_sftp_entries(
        &mut self,
        host_id: HostId,
        current_dir: impl Into<String>,
        entries: Vec<SftpEntry>,
    ) -> bool {
        if let Some(browser) = self
            .sftp_browsers
            .iter_mut()
            .find(|browser| browser.host_id == host_id)
        {
            browser.current_dir = current_dir.into();
            browser.entries = entries;
            browser.loading = false;
            browser.last_error = None;
            true
        } else {
            false
        }
    }

    /// 按会话标签页更新 SFTP 当前目录和目录项。
    pub fn set_sftp_entries_for_session(
        &mut self,
        session_id: SessionId,
        current_dir: impl Into<String>,
        entries: Vec<SftpEntry>,
    ) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .and_then(|tab| tab.host_id)
            .map(|host_id| self.set_sftp_entries(host_id, current_dir, entries))
            .unwrap_or(false)
    }

    /// 记录 SFTP 浏览错误。
    pub fn fail_sftp_browser(&mut self, host_id: HostId, reason: impl Into<String>) -> bool {
        if let Some(browser) = self
            .sftp_browsers
            .iter_mut()
            .find(|browser| browser.host_id == host_id)
        {
            browser.loading = false;
            browser.last_error = Some(reason.into());
            true
        } else {
            false
        }
    }

    /// 按会话标签页记录 SFTP 浏览错误。
    pub fn fail_sftp_browser_for_session(
        &mut self,
        session_id: SessionId,
        reason: impl Into<String>,
    ) -> bool {
        self.tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .and_then(|tab| tab.host_id)
            .map(|host_id| self.fail_sftp_browser(host_id, reason))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SftpEntryKind;
    use uuid::Uuid;

    fn host_id() -> HostId {
        HostId(Uuid::new_v4())
    }

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    #[test]
    fn opening_sftp_tab_creates_browser_state() {
        let mut sessions = SessionManager::default();
        let id = session_id();
        let host_id = host_id();

        sessions.open_sftp_tab(id, host_id, "/home/ops");

        assert_eq!(sessions.tab_count(), 1);
        assert_eq!(sessions.sftp_browser_count(), 1);
        assert_eq!(sessions.sftp_browsers[0].host_id, host_id);
        assert_eq!(sessions.sftp_browsers[0].current_dir, "/home/ops");
        assert!(matches!(sessions.tabs[0].kind, SessionKind::Sftp));
    }

    #[test]
    fn sftp_entries_update_existing_browser() {
        let mut sessions = SessionManager::default();
        let host_id = host_id();

        sessions.open_sftp_tab(session_id(), host_id, "/home/ops");

        assert!(sessions.set_sftp_entries(
            host_id,
            "/var/log",
            vec![SftpEntry {
                name: "syslog".to_owned(),
                remote_path: "/var/log/syslog".to_owned(),
                kind: SftpEntryKind::File,
                size: Some(100),
                modified_at_unix_secs: None,
                permissions: None,
            }],
        ));
        assert_eq!(sessions.sftp_browsers[0].current_dir, "/var/log");
        assert_eq!(sessions.sftp_browsers[0].entries.len(), 1);
        assert!(!sessions.sftp_browsers[0].loading);
        assert!(sessions.sftp_browsers[0].last_error.is_none());
    }

    #[test]
    fn sftp_entries_can_update_by_session_id() {
        let mut sessions = SessionManager::default();
        let host_id = host_id();
        let session_id = session_id();

        sessions.open_sftp_tab(session_id, host_id, "/home/ops");

        assert!(sessions.set_sftp_entries_for_session(session_id, "/tmp", Vec::new()));
        assert_eq!(sessions.sftp_browsers[0].current_dir, "/tmp");
        assert!(!sessions.set_sftp_entries_for_session(
            SessionId(Uuid::new_v4()),
            "/missing",
            Vec::new()
        ));
    }

    #[test]
    fn sftp_browser_records_failure() {
        let mut sessions = SessionManager::default();
        let current_host_id = host_id();

        sessions.open_sftp_tab(session_id(), current_host_id, "/home/ops");

        assert!(sessions.fail_sftp_browser(current_host_id, "permission denied"));
        assert_eq!(
            sessions.sftp_browsers[0].last_error.as_deref(),
            Some("permission denied")
        );
        assert!(!sessions.fail_sftp_browser(host_id(), "missing"));
    }

    #[test]
    fn sftp_failure_can_update_by_session_id() {
        let mut sessions = SessionManager::default();
        let host_id = host_id();
        let session_id = session_id();

        sessions.open_sftp_tab(session_id, host_id, "/home/ops");

        assert!(sessions.fail_sftp_browser_for_session(session_id, "network"));
        assert_eq!(
            sessions.sftp_browsers[0].last_error.as_deref(),
            Some("network")
        );
    }
}
