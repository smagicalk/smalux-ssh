//! SFTP 下载调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{HostId, TransferDirection, TransferId, TransferStatus, TransferTask};

use super::super::launch::queued_outcome;
use super::super::launch_sftp::missing_active_sftp_session;
use super::super::{AppState, AppUpdateOutcome};
use super::path::basename_local_path;

impl AppState {
    /// 将当前远程文件下载到本地路径草稿。
    pub(in crate::model::app_state) fn download_sftp(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = remote_path.trim().to_owned();
        if remote_path.is_empty() || remote_path == "/" {
            return AppUpdateOutcome {
                error: Some("SFTP 下载路径不能为空或根目录".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let local_path = self.ui.sftp_local_path_for(host_id).trim().to_owned();
        let local_path = if local_path.is_empty() {
            match basename_local_path(&remote_path) {
                Some(name) => name,
                None => {
                    return AppUpdateOutcome {
                        error: Some("SFTP 本地路径不能为空".to_owned()),
                        ..AppUpdateOutcome::default()
                    };
                }
            }
        } else {
            local_path
        };
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
