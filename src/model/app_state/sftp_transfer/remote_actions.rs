//! SFTP 远端路径操作调度。

use crate::backend::SftpRequest;
use crate::core::CoreState;
use crate::model::HostId;

use super::super::AppUpdateOutcome;
use super::super::launch::join_remote_path;
use super::super::launch_sftp::session::missing_sftp_browser;
use super::path::is_plain_remote_name;

impl CoreState {
    /// 删除远程文件。
    pub(in crate::model::app_state) fn remove_sftp_file(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = remote_path.trim().to_owned();
        if remote_path.is_empty() || remote_path == "/" {
            return AppUpdateOutcome {
                error: Some("SFTP 删除路径不能为空或根目录".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.queue_sftp_path_action(host_id, SftpRequest::RemoveFile { remote_path })
    }

    /// 创建 SFTP 远端目录的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn create_sftp_dir_named_action(
        &mut self,
        host_id: HostId,
        new_dir_name: String,
    ) -> AppUpdateOutcome {
        self.create_sftp_dir_named(host_id, new_dir_name)
    }

    /// 在当前远程目录创建子目录。
    pub(in crate::model::app_state) fn create_sftp_dir_named(
        &mut self,
        host_id: HostId,
        new_dir_name: String,
    ) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        if new_dir_name.is_empty() {
            return AppUpdateOutcome {
                error: Some("SFTP 新目录名不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if !is_plain_remote_name(&new_dir_name) {
            return AppUpdateOutcome {
                error: Some("SFTP 新目录名不能包含路径分隔符".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let remote_path = join_remote_path(&current_dir, &new_dir_name);
        self.queue_sftp_path_action(host_id, SftpRequest::CreateDir { remote_path })
    }
}
