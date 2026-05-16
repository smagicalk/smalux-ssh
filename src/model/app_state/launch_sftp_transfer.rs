//! SFTP 上传、下载和远端路径操作调度。

use std::path::Path;

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{HostId, TransferDirection, TransferId, TransferStatus, TransferTask};

use super::launch::{join_remote_path, queued_outcome};
use super::launch_sftp::missing_sftp_browser;
use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 将本地文件上传到当前 SFTP 目录。
    pub(super) fn upload_sftp(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(session_id) = self.sftp_session_id_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };
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
        let remote_path = join_remote_path(&current_dir, &remote_name);
        let transfer_id = TransferId(Uuid::new_v4());

        self.sessions.enqueue_transfer(TransferTask {
            id: transfer_id,
            host_id,
            direction: TransferDirection::Upload,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        });
        self.sessions.set_sftp_loading(host_id, true);
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

    /// 将当前远程文件下载到本地路径草稿。
    pub(super) fn download_sftp(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let Some(session_id) = self.sftp_session_id_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

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
        let transfer_id = TransferId(Uuid::new_v4());

        self.sessions.enqueue_transfer(TransferTask {
            id: transfer_id,
            host_id,
            direction: TransferDirection::Download,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        });
        self.sessions.set_sftp_loading(host_id, true);
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

    /// 删除远程文件。
    pub(super) fn remove_sftp_file(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        self.queue_sftp_path_action(host_id, SftpRequest::RemoveFile { remote_path })
    }

    /// 在当前远程目录创建子目录。
    pub(super) fn create_sftp_dir(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        let new_dir_name = self.ui.sftp_new_dir_name_for(host_id).trim().to_owned();
        if new_dir_name.is_empty() {
            return AppUpdateOutcome {
                error: Some("SFTP 新目录名不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let remote_path = join_remote_path(&current_dir, &new_dir_name);
        self.queue_sftp_path_action(host_id, SftpRequest::CreateDir { remote_path })
    }
}

fn basename_local_path(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(ToOwned::to_owned)
}
