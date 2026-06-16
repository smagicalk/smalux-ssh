//! 桌面层 SFTP 草稿提交逻辑。

use crate::model::{AppUpdateOutcome, HostId};

use super::{DesktopAppState, basename_local_path};

impl DesktopAppState {
    pub(super) fn upload_sftp_local(&mut self, host_id: HostId) -> AppUpdateOutcome {
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

        self.core
            .upload_sftp_with_paths_action(host_id, local_path, remote_name)
    }

    pub(super) fn download_sftp_local(
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

        self.core
            .download_sftp_to_path_action(host_id, remote_path, local_path)
    }

    pub(super) fn create_sftp_dir_local(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let new_dir_name = self.ui.sftp_new_dir_name_for(host_id).trim().to_owned();
        self.core
            .create_sftp_dir_named_action(host_id, new_dir_name)
    }
}
