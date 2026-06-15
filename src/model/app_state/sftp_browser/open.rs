//! SFTP 浏览器目录导航和选择调度。

use crate::backend::{BackendCommand, SftpRequest};
use crate::core::CoreState;
use crate::model::HostId;

use super::super::AppUpdateOutcome;
use super::super::launch::{normalize_remote_dir, queued_outcome};
use super::session::{missing_active_sftp_session, missing_sftp_browser};

#[path = "open_start.rs"]
mod open_start;

impl CoreState {
    /// 刷新当前 SFTP 目录。
    pub(in crate::model::app_state) fn refresh_sftp(
        &mut self,
        host_id: HostId,
    ) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        self.queue_sftp_list_dir(host_id, current_dir)
    }

    /// 切换到指定 SFTP 目录。
    pub(in crate::model::app_state) fn navigate_sftp(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = normalize_remote_dir(&remote_path);

        self.queue_sftp_list_dir(host_id, remote_path)
    }

    /// 记录当前选中的 SFTP 目录项，不触发后端请求。
    pub(in crate::model::app_state) fn select_sftp_entry(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        if self.claim_sftp_session_id_for_host(host_id).is_none() {
            return missing_active_sftp_session(host_id);
        }

        if self.sessions.select_sftp_entry(host_id, remote_path) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            missing_sftp_browser(host_id)
        }
    }

    pub(in crate::model::app_state) fn queue_sftp_path_action(
        &mut self,
        host_id: HostId,
        request: SftpRequest,
    ) -> AppUpdateOutcome {
        let Some(session_id) = self.claim_sftp_session_id_for_host(host_id) else {
            return missing_active_sftp_session(host_id);
        };

        self.sessions.set_sftp_loading_for_session(session_id, true);
        self.backend_commands.push(BackendCommand::Sftp {
            session_id,
            request,
        });

        queued_outcome(1)
    }

    fn queue_sftp_list_dir(&mut self, host_id: HostId, remote_path: String) -> AppUpdateOutcome {
        self.queue_sftp_path_action(host_id, SftpRequest::ListDir { remote_path })
    }

    pub(in crate::model::app_state) fn current_sftp_dir_for_host(
        &self,
        host_id: HostId,
    ) -> Option<String> {
        self.sessions
            .sftp_browsers
            .iter()
            .find(|browser| browser.host_id == host_id)
            .map(|browser| browser.current_dir.clone())
    }
}
