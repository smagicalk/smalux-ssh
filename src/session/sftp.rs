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
            session_id: id,
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
            let selected_still_visible = browser
                .selected_path
                .as_ref()
                .map(|selected_path| {
                    entries
                        .iter()
                        .any(|entry| entry.remote_path == *selected_path)
                })
                .unwrap_or(true);
            browser.current_dir = current_dir.into();
            browser.entries = entries;
            if !selected_still_visible {
                browser.selected_path = None;
            }
            browser.loading = false;
            browser.last_error = None;
            true
        } else {
            false
        }
    }

    /// 设置 SFTP 浏览器加载状态。
    pub fn set_sftp_loading(&mut self, host_id: HostId, loading: bool) -> bool {
        if let Some(browser) = self
            .sftp_browsers
            .iter_mut()
            .find(|browser| browser.host_id == host_id)
        {
            browser.loading = loading;
            if loading {
                browser.last_error = None;
            }
            true
        } else {
            false
        }
    }

    /// 按会话标签页设置 SFTP 浏览器加载状态。
    pub fn set_sftp_loading_for_session(&mut self, session_id: SessionId, loading: bool) -> bool {
        let Some(host_id) = self
            .tabs
            .iter()
            .find(|tab| tab.id == session_id && matches!(tab.kind, SessionKind::Sftp))
            .and_then(|tab| tab.host_id)
        else {
            return false;
        };
        if !self.sftp_browser_belongs_to_session(host_id, session_id) {
            return false;
        }

        self.set_sftp_loading(host_id, loading)
    }

    /// 记录当前选中的 SFTP 目录项。
    pub fn select_sftp_entry(&mut self, host_id: HostId, selected_path: impl Into<String>) -> bool {
        if let Some(browser) = self
            .sftp_browsers
            .iter_mut()
            .find(|browser| browser.host_id == host_id)
        {
            browser.selected_path = Some(selected_path.into());
            true
        } else {
            false
        }
    }

    /// 清空当前 SFTP 选中项。
    pub fn clear_sftp_selection(&mut self, host_id: HostId) -> bool {
        if let Some(browser) = self
            .sftp_browsers
            .iter_mut()
            .find(|browser| browser.host_id == host_id)
        {
            browser.selected_path = None;
            true
        } else {
            false
        }
    }

    /// 将 SFTP 浏览器归属转交给指定会话。
    pub fn reassign_sftp_browser_session(
        &mut self,
        host_id: HostId,
        session_id: SessionId,
    ) -> bool {
        if !self.session_is_sftp_tab_for_host(session_id, host_id) {
            return false;
        }

        let Some(browser) = self
            .sftp_browsers
            .iter_mut()
            .find(|browser| browser.host_id == host_id)
        else {
            return false;
        };

        if browser.session_id == session_id {
            return false;
        }

        browser.session_id = session_id;
        true
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
            .find(|tab| {
                tab.id == session_id
                    && matches!(tab.kind, SessionKind::Sftp)
                    && sftp_tab_can_accept_browser_owner(&tab.status)
            })
            .and_then(|tab| tab.host_id)
            .filter(|host_id| self.sftp_browser_belongs_to_session(*host_id, session_id))
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
        let reason = reason.into();

        self.tabs
            .iter()
            .find(|tab| tab.id == session_id && matches!(tab.kind, SessionKind::Sftp))
            .and_then(|tab| tab.host_id)
            .filter(|host_id| self.sftp_browser_belongs_to_session(*host_id, session_id))
            .map(|host_id| self.fail_sftp_browser(host_id, reason))
            .unwrap_or(false)
    }

    /// 按会话标签页记录 SFTP 操作错误，忽略已经失效的迟到操作结果。
    pub fn fail_sftp_operation_for_session(
        &mut self,
        session_id: SessionId,
        reason: impl Into<String>,
    ) -> bool {
        let reason = reason.into();

        self.tabs
            .iter()
            .find(|tab| {
                tab.id == session_id
                    && matches!(tab.kind, SessionKind::Sftp)
                    && sftp_tab_can_accept_browser_owner(&tab.status)
            })
            .and_then(|tab| tab.host_id)
            .filter(|host_id| self.sftp_browser_belongs_to_session(*host_id, session_id))
            .map(|host_id| self.fail_sftp_browser(host_id, reason))
            .unwrap_or(false)
    }

    /// 当前 SFTP owner 失效后，转交给同主机可用 SFTP 会话。
    pub fn reassign_sftp_browser_after_session_loss(&mut self, session_id: SessionId) -> bool {
        let Some((host_id, owner_lost)) = self
            .tabs
            .iter()
            .find(|tab| tab.id == session_id && matches!(tab.kind, SessionKind::Sftp))
            .and_then(|tab| {
                tab.host_id
                    .map(|host_id| (host_id, !sftp_tab_can_accept_browser_owner(&tab.status)))
            })
        else {
            return false;
        };
        if !owner_lost || !self.sftp_browser_belongs_to_session(host_id, session_id) {
            return false;
        }

        let Some(next_session_id) = self
            .tabs
            .iter()
            .rev()
            .find(|tab| {
                tab.id != session_id
                    && tab.host_id == Some(host_id)
                    && matches!(tab.kind, SessionKind::Sftp)
                    && sftp_tab_can_accept_browser_owner(&tab.status)
            })
            .map(|tab| tab.id)
        else {
            return false;
        };

        let Some(browser) = self
            .sftp_browsers
            .iter_mut()
            .find(|browser| browser.host_id == host_id && browser.session_id == session_id)
        else {
            return false;
        };

        browser.session_id = next_session_id;
        browser.loading = false;
        browser.last_error = None;
        true
    }

    fn sftp_browser_belongs_to_session(&self, host_id: HostId, session_id: SessionId) -> bool {
        self.sftp_browsers
            .iter()
            .any(|browser| browser.host_id == host_id && browser.session_id == session_id)
    }

    fn session_is_sftp_tab_for_host(&self, session_id: SessionId, host_id: HostId) -> bool {
        self.tabs.iter().any(|tab| {
            tab.id == session_id
                && tab.host_id == Some(host_id)
                && matches!(tab.kind, SessionKind::Sftp)
                && sftp_tab_can_accept_browser_owner(&tab.status)
        })
    }
}

fn sftp_tab_can_accept_browser_owner(status: &SessionStatus) -> bool {
    !matches!(
        status,
        SessionStatus::Disconnected | SessionStatus::Failed { .. }
    )
}

#[cfg(test)]
mod tests;
