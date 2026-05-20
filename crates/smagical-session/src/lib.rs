//! 运行中 SSH 会话管理。
//!
//! 这里只记录会话索引、标签页和生命周期状态；实际网络连接、PTY、SFTP 和隧道任务
//! 会拆到独立服务模块，避免 UI 状态持有不可序列化的连接细节。

mod sftp;
mod tabs;
mod transfers;
mod tunnels;

use smagical_core::{SessionId, SessionTab, SftpBrowserState, TransferTask, TunnelRuntimeState};

/// 当前进程内的活动会话集合。
#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    pub active: Vec<SessionId>,
    pub tabs: Vec<SessionTab>,
    pub active_tab: Option<SessionId>,
    pub tunnels: Vec<TunnelRuntimeState>,
    pub sftp_browsers: Vec<SftpBrowserState>,
    pub transfers: Vec<TransferTask>,
}

impl SessionManager {
    /// 活动连接数量。
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// 标签页数量。
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// 运行中或已登记的隧道数量。
    pub fn tunnel_runtime_count(&self) -> usize {
        self.tunnels.len()
    }

    /// SFTP 浏览器数量。
    pub fn sftp_browser_count(&self) -> usize {
        self.sftp_browsers.len()
    }

    /// SFTP 传输任务数量。
    pub fn transfer_count(&self) -> usize {
        self.transfers.len()
    }

    pub(crate) fn push_tab(&mut self, tab: SessionTab) {
        self.active.retain(|active_id| *active_id != tab.id);
        self.tabs.retain(|existing| existing.id != tab.id);
        self.active.push(tab.id);
        self.active_tab = Some(tab.id);
        self.tabs.push(tab);
    }

    pub(crate) fn upsert_tunnel(&mut self, tunnel: TunnelRuntimeState) {
        if let Some(existing) = self.tunnels.iter_mut().find(|existing| {
            existing.session_id == tunnel.session_id && existing.rule_name == tunnel.rule_name
        }) {
            *existing = tunnel;
        } else {
            self.tunnels.push(tunnel);
        }
    }

    /// 保存或替换指定主机的 SFTP 浏览器状态。
    pub fn upsert_sftp_browser(&mut self, browser: SftpBrowserState) {
        if let Some(existing) = self
            .sftp_browsers
            .iter_mut()
            .find(|existing| existing.host_id == browser.host_id)
        {
            *existing = browser;
        } else {
            self.sftp_browsers.push(browser);
        }
    }

    pub(crate) fn update_tunnel(
        &mut self,
        session_id: SessionId,
        rule_name: &str,
        update: impl FnOnce(&mut TunnelRuntimeState),
    ) -> bool {
        if let Some(tunnel) = self
            .tunnels
            .iter_mut()
            .find(|tunnel| tunnel.session_id == session_id && tunnel.rule_name == rule_name)
        {
            update(tunnel);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn default_session_manager_has_no_active_sessions() {
        let sessions = SessionManager::default();

        assert_eq!(sessions.active_count(), 0);
        assert_eq!(sessions.tab_count(), 0);
        assert!(sessions.active_tab.is_none());
    }

    #[test]
    fn active_count_tracks_sessions_added_by_runtime() {
        let mut sessions = SessionManager::default();

        sessions.active.push(SessionId(Uuid::new_v4()));
        sessions.active.push(SessionId(Uuid::new_v4()));

        assert_eq!(sessions.active_count(), 2);
    }
}
