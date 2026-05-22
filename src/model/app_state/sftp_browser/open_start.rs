//! SFTP 浏览器打开调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{HostId, SessionId, SessionStatus, WorkspacePage};

use super::super::super::launch::{
    connect_command_with_known_hosts, missing_host, normalize_remote_dir, queued_outcome,
};
use super::super::super::{AppState, AppUpdateOutcome};

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
}
