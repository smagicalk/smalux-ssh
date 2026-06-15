//! SFTP 上传调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::core::CoreState;
use crate::model::{HostId, TransferDirection, TransferId, TransferStatus, TransferTask};

use super::super::AppUpdateOutcome;
use super::super::launch::{join_remote_path, queued_outcome};
use super::super::launch_sftp::session::{missing_active_sftp_session, missing_sftp_browser};
use super::path::is_plain_remote_name;

impl CoreState {
    /// 上传 SFTP 文件的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn upload_sftp_with_paths_action(
        &mut self,
        host_id: HostId,
        local_path: String,
        remote_name: String,
    ) -> AppUpdateOutcome {
        self.upload_sftp_with_paths(host_id, local_path, remote_name)
    }

    /// 将本地文件上传到当前 SFTP 目录。
    pub(in crate::model::app_state) fn upload_sftp_with_paths(
        &mut self,
        host_id: HostId,
        local_path: String,
        remote_name: String,
    ) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };
        if local_path.is_empty() {
            return AppUpdateOutcome {
                error: Some("SFTP 本地路径不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if !is_plain_remote_name(&remote_name) {
            return AppUpdateOutcome {
                error: Some("SFTP 远程文件名不能包含路径分隔符".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        let Some(session_id) = self.claim_sftp_session_id_for_host(host_id) else {
            return missing_active_sftp_session(host_id);
        };
        let remote_path = join_remote_path(&current_dir, &remote_name);
        let transfer_id = TransferId(Uuid::new_v4());

        self.sessions.enqueue_transfer(TransferTask {
            id: transfer_id,
            session_id,
            host_id,
            direction: TransferDirection::Upload,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        });
        self.sessions.set_sftp_loading_for_session(session_id, true);
        self.backend_commands.push(BackendCommand::Sftp {
            session_id,
            request: SftpRequest::Upload {
                id: transfer_id,
                local_path,
                remote_path,
            },
        });

        queued_outcome(1)
    }
}
