//! SFTP 下载调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::core::CoreState;
use crate::model::{HostId, TransferDirection, TransferId, TransferStatus, TransferTask};

use super::super::AppUpdateOutcome;
use super::super::launch::queued_outcome;
use super::super::launch_sftp::session::missing_active_sftp_session;

impl CoreState {
    /// 下载 SFTP 文件的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn download_sftp_to_path_action(
        &mut self,
        host_id: HostId,
        remote_path: String,
        local_path: String,
    ) -> AppUpdateOutcome {
        self.download_sftp_to_path(host_id, remote_path, local_path)
    }

    /// 将远程文件下载到指定本地路径。
    pub(in crate::model::app_state) fn download_sftp_to_path(
        &mut self,
        host_id: HostId,
        remote_path: String,
        local_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = remote_path.trim().to_owned();
        if remote_path.is_empty() || remote_path == "/" {
            return AppUpdateOutcome {
                error: Some("SFTP 下载路径不能为空或根目录".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if local_path.is_empty() {
            return AppUpdateOutcome {
                error: Some("SFTP 本地路径不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        let Some(session_id) = self.claim_sftp_session_id_for_host(host_id) else {
            return missing_active_sftp_session(host_id);
        };
        let transfer_id = TransferId(Uuid::new_v4());

        self.sessions.enqueue_transfer(TransferTask {
            id: transfer_id,
            session_id,
            host_id,
            direction: TransferDirection::Download,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        });
        self.backend_commands.push(BackendCommand::Sftp {
            session_id,
            request: SftpRequest::Download {
                id: transfer_id,
                remote_path,
                local_path,
            },
        });

        queued_outcome(1)
    }
}
