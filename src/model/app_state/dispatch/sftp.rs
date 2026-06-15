//! SFTP 浏览器和传输消息路由。
//!
//! 这里处理一个已打开 SFTP 浏览器之后的操作：刷新、导航、书签、上传、下载、
//! 取消传输和远端文件操作。打开 SFTP 会话本身属于 launch 路由。

use std::path::Path;

use crate::core::CoreState;

use super::super::{AppState, AppUpdateOutcome, Message};

impl CoreState {
    /// 分发 SFTP 运行期消息。
    ///
    /// 大多数分支会先校验当前浏览器归属，再决定是否排队后端请求；这样 UI 不需要
    /// 关心哪个 session 当前真正拥有 SFTP 浏览器。
    pub(super) fn dispatch_sftp_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::RefreshSftp { host_id } => self.refresh_sftp(host_id),
            Message::SaveSftpBookmark { host_id } => self.save_sftp_bookmark(host_id),
            Message::OpenSftpBookmark {
                host_id,
                remote_path,
            } => self.open_sftp_bookmark(host_id, remote_path),
            Message::RemoveSftpBookmark {
                host_id,
                remote_path,
            } => self.remove_sftp_bookmark(host_id, remote_path),
            Message::NavigateSftp {
                host_id,
                remote_path,
            } => self.navigate_sftp(host_id, remote_path),
            Message::SelectSftpEntry {
                host_id,
                remote_path,
            } => self.select_sftp_entry(host_id, remote_path),
            Message::CancelSftpTransfer { transfer_id } => self.cancel_sftp_transfer(transfer_id),
            Message::RemoveSftpFile {
                host_id,
                remote_path,
            } => self.remove_sftp_file(host_id, remote_path),
            Message::UploadSftp { .. }
            | Message::DownloadSftp { .. }
            | Message::CreateSftpDir { .. } => AppUpdateOutcome {
                error: Some(
                    "当前 SFTP 消息仍依赖桌面草稿状态，不能只在 CoreState 中运行".to_owned(),
                ),
                ..AppUpdateOutcome::default()
            },
            _ => unreachable!("非 SFTP 消息不应进入 SFTP 路由"),
        }
    }
}

impl AppState {
    pub(super) fn dispatch_sftp_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::RefreshSftp { .. }
            | Message::SaveSftpBookmark { .. }
            | Message::OpenSftpBookmark { .. }
            | Message::RemoveSftpBookmark { .. }
            | Message::NavigateSftp { .. }
            | Message::SelectSftpEntry { .. }
            | Message::CancelSftpTransfer { .. }
            | Message::RemoveSftpFile { .. } => self.core.dispatch_sftp_message(message),
            _ => match message {
                Message::UploadSftp { host_id } => self.core.upload_sftp_with_paths_action(
                    host_id,
                    self.ui.sftp_local_path_for(host_id).trim().to_owned(),
                    {
                        let remote_name = self.ui.sftp_remote_name_for(host_id).trim();
                        if remote_name.is_empty() {
                            basename_local_path(self.ui.sftp_local_path_for(host_id).trim())
                                .unwrap_or_default()
                        } else {
                            remote_name.to_owned()
                        }
                    },
                ),
                Message::DownloadSftp {
                    host_id,
                    remote_path,
                } => self
                    .core
                    .download_sftp_to_path_action(host_id, remote_path.clone(), {
                        let local_path = self.ui.sftp_local_path_for(host_id).trim().to_owned();
                        if local_path.is_empty() {
                            basename_local_path(&remote_path).unwrap_or_default()
                        } else {
                            local_path
                        }
                    }),
                Message::CreateSftpDir { host_id } => self.core.create_sftp_dir_named_action(
                    host_id,
                    self.ui.sftp_new_dir_name_for(host_id).trim().to_owned(),
                ),
                _ => unreachable!("非 SFTP 消息不应进入 SFTP 路由"),
            },
        }
    }
}

fn basename_local_path(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(ToOwned::to_owned)
}
