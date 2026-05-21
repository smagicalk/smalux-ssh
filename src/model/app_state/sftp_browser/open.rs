//! SFTP 浏览器打开和目录导航调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{HostId, SessionId, SessionStatus, WorkspacePage};

use super::super::launch::{
    connect_command_with_known_hosts, missing_host, normalize_remote_dir, queued_outcome,
};
use super::super::{AppState, AppUpdateOutcome};
use super::session::{missing_active_sftp_session, missing_sftp_browser};

impl AppState {
    /// 打开 SFTP 浏览器，并排队读取初始远端目录。
    pub(in crate::model::app_state) fn open_sftp(
        &mut self,
        host_id: HostId,
        initial_dir: String,
    ) -> AppUpdateOutcome {
        let initial_dir = normalize_remote_dir(&initial_dir);
        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };

        let session_id = SessionId(Uuid::new_v4());
        self.sessions
            .open_sftp_tab(session_id, host.id, initial_dir.clone());
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.sessions.set_sftp_loading(host.id, true);
        self.ui.workspace.active_page = WorkspacePage::Sftp;
        self.record_recent_connection(&host);
        let known_hosts = self.storage.known_hosts.clone();
        self.backend_commands.extend([
            connect_command_with_known_hosts(session_id, &host, known_hosts),
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::ListDir {
                    remote_path: initial_dir,
                },
            },
        ]);

        queued_outcome(2)
    }

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
