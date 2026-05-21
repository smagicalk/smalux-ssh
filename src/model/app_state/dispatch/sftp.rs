//! SFTP 浏览器和传输消息路由。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
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
            Message::UploadSftp { host_id } => self.upload_sftp(host_id),
            Message::DownloadSftp {
                host_id,
                remote_path,
            } => self.download_sftp(host_id, remote_path),
            Message::CancelSftpTransfer { transfer_id } => self.cancel_sftp_transfer(transfer_id),
            Message::RemoveSftpFile {
                host_id,
                remote_path,
            } => self.remove_sftp_file(host_id, remote_path),
            Message::CreateSftpDir { host_id } => self.create_sftp_dir(host_id),
            _ => unreachable!("非 SFTP 消息不应进入 SFTP 路由"),
        }
    }
}
