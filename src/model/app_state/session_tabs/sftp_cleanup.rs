//! SFTP browser 在标签页关闭后的归属清理。

use crate::model::{SessionKind, SessionTab};

use super::super::AppState;
use super::activate::sftp_tab_can_accept_browser_owner;

impl AppState {
    pub(super) fn remove_sftp_browser_after_tab_close(&mut self, tab: &SessionTab) -> bool {
        if !matches!(tab.kind, SessionKind::Sftp) {
            return false;
        }

        let Some(host_id) = tab.host_id else {
            return false;
        };
        let has_other_sftp_tab = self.sessions.tabs.iter().any(|other| {
            other.host_id == Some(host_id)
                && matches!(other.kind, SessionKind::Sftp)
                && sftp_tab_can_accept_browser_owner(&other.status)
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
                    && sftp_tab_can_accept_browser_owner(&other.status)
            })
            .map(|other| other.id)
        else {
            return false;
        };

        browser.session_id = next_session_id;
        browser.loading = false;
        browser.last_error = None;
        true
    }
}
