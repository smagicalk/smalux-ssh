//! SFTP 书签调度。

use crate::model::{HostId, SftpBookmark};

use super::super::launch::normalize_remote_dir;
use super::super::{AppState, AppUpdateOutcome};
use super::session::{SftpCurrentDirLookup, missing_active_sftp_session, missing_sftp_browser};

impl AppState {
    /// 将当前 SFTP 浏览目录保存为书签。
    pub(in crate::model::app_state) fn save_sftp_bookmark(
        &mut self,
        host_id: HostId,
    ) -> AppUpdateOutcome {
        let current_dir = match self.claimed_current_sftp_dir_for_host(host_id) {
            SftpCurrentDirLookup::Found(current_dir) => current_dir,
            SftpCurrentDirLookup::MissingBrowser => return missing_sftp_browser(host_id),
            SftpCurrentDirLookup::MissingActiveSession => {
                return missing_active_sftp_session(host_id);
            }
        };

        self.storage.upsert_sftp_bookmark(SftpBookmark {
            host_id,
            label: sftp_bookmark_label(&current_dir),
            remote_path: current_dir,
        });

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 打开 SFTP 书签；已有浏览器时导航，否则新开 SFTP 标签页。
    pub(in crate::model::app_state) fn open_sftp_bookmark(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = normalize_remote_dir(&remote_path);

        if self.sftp_session_id_for_host(host_id).is_some() {
            self.queue_sftp_path_action(
                host_id,
                crate::backend::SftpRequest::ListDir { remote_path },
            )
        } else {
            self.open_sftp(host_id, remote_path)
        }
    }

    /// 删除指定 SFTP 书签。
    pub(in crate::model::app_state) fn remove_sftp_bookmark(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        if self.storage.remove_sftp_bookmark(host_id, &remote_path) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到 SFTP 书签：{remote_path}")),
                ..AppUpdateOutcome::default()
            }
        }
    }
}

fn sftp_bookmark_label(remote_path: &str) -> String {
    if remote_path == "/" {
        return "/".to_owned();
    }

    remote_path
        .trim_end_matches('/')
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(remote_path)
        .to_owned()
}
