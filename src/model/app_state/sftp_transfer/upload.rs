//! SFTP 上传调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{HostId, TransferDirection, TransferId, TransferStatus, TransferTask};

use super::super::launch::{join_remote_path, queued_outcome};
use super::super::launch_sftp::session::{missing_active_sftp_session, missing_sftp_browser};
use super::super::{AppState, AppUpdateOutcome};
use super::path::{basename_local_path, is_plain_remote_name};

impl AppState {
    /// 将本地文件上传到当前 SFTP 目录。
    pub(in crate::model::app_state) fn upload_sftp(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        let local_path = self.ui.sftp_local_path_for(host_id).trim().to_owned();
        if local_path.is_empty() {
            return AppUpdateOutcome {
                error: Some("SFTP 本地路径不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let remote_name = self.ui.sftp_remote_name_for(host_id).trim();
        let remote_name = if remote_name.is_empty() {
            match basename_local_path(&local_path) {
                Some(name) => name,
                None => {
                    return AppUpdateOutcome {
                        error: Some("无法从本地路径推断远程文件名".to_owned()),
                        ..AppUpdateOutcome::default()
                    };
                }
            }
        } else {
            remote_name.to_owned()
        };
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
