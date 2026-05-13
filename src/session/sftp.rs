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
        let reason = reason.into();

        self.tabs
            .iter()
            .find(|tab| tab.id == session_id && matches!(tab.kind, SessionKind::Sftp))
            .and_then(|tab| tab.host_id)
            .map(|host_id| self.fail_sftp_browser(host_id, reason))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests;
