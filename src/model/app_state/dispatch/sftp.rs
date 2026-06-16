//! SFTP 浏览器和传输消息路由。
//!
//! 这里处理一个已打开 SFTP 浏览器之后的操作：刷新、导航、书签、上传、下载、
//! 取消传输和远端文件操作。打开 SFTP 会话本身属于 launch 路由。

use super::super::{AppUpdateOutcome, Message};
use crate::core::CoreState;

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
