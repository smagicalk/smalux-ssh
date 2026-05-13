//! SSH SFTP 子系统会话。

use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileAttributes, FileType as RusshSftpFileType};
use tokio::io::AsyncWriteExt;

use crate::backend::{BackendEvent, BackendExecutionError, SftpRequest};
use crate::model::{SessionId, SftpEntry, SftpEntryKind, TransferStatus};

use super::super::RusshConnection;
use super::{channel_error, wait_channel_request};

/// 已打开的远程 SFTP 子系统会话。
pub struct RemoteSftp {
    session_id: SessionId,
    session: SftpSession,
}

impl RemoteSftp {
    /// 返回 SFTP 会话关联的 UI 会话标识。
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// 执行一个 SFTP 请求并转换成 UI 事件。
    pub async fn execute(
        &self,
        request: SftpRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        match request {
            SftpRequest::ListDir { remote_path } => self.list_dir(remote_path).await,
            SftpRequest::Upload {
                id,
                local_path,
                remote_path,
            } => self.upload(id, local_path, remote_path).await,
            SftpRequest::Download {
                id,
                remote_path,
                local_path,
            } => self.download(id, remote_path, local_path).await,
            SftpRequest::RemoveFile { remote_path } => {
                self.session
                    .remove_file(remote_path.clone())
                    .await
                    .map_err(|error| sftp_error("remove file", error))?;
                self.list_dir(parent_remote_dir(&remote_path)).await
            }
            SftpRequest::CreateDir { remote_path } => {
                self.session
                    .create_dir(remote_path.clone())
                    .await
                    .map_err(|error| sftp_error("create dir", error))?;
                self.list_dir(parent_remote_dir(&remote_path)).await
            }
        }
    }

    /// 主动关闭 SFTP 子系统。
    pub async fn close(&self) -> Result<(), BackendExecutionError> {
        self.session
            .close()
            .await
            .map_err(|error| sftp_error("close", error))
    }

    async fn list_dir(
        &self,
        remote_path: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let entries = self
            .session
            .read_dir(remote_path.clone())
            .await
            .map_err(|error| sftp_error("list dir", error))?
            .map(|entry| sftp_entry_from_parts(&remote_path, entry.file_name(), entry.metadata()))
            .collect();

        Ok(vec![BackendEvent::SftpEntries {
            session_id: self.session_id,
            remote_path,
            entries,
        }])
    }

    async fn upload(
        &self,
        id: crate::model::TransferId,
        local_path: String,
        remote_path: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let data = tokio::fs::read(&local_path)
            .await
            .map_err(|error| sftp_io_error("upload read local", error))?;
        let total_bytes = data.len() as u64;
        let mut remote_file = self
            .session
            .create(remote_path.clone())
            .await
            .map_err(|error| sftp_error("upload open remote", error))?;

        remote_file
            .write_all(&data)
            .await
            .map_err(|error| sftp_io_error("upload write remote", error))?;
        remote_file
            .shutdown()
            .await
            .map_err(|error| sftp_io_error("upload close remote", error))?;

        let mut events = transfer_events(self.session_id, id, total_bytes);
        events.extend(self.list_dir(parent_remote_dir(&remote_path)).await?);
        Ok(events)
    }

    async fn download(
        &self,
        id: crate::model::TransferId,
        remote_path: String,
        local_path: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let data = self
            .session
            .read(remote_path)
            .await
            .map_err(|error| sftp_error("download read remote", error))?;
        tokio::fs::write(&local_path, &data)
            .await
            .map_err(|error| sftp_io_error("download write local", error))?;

        Ok(transfer_events(self.session_id, id, data.len() as u64))
    }
}

impl RusshConnection {
    /// 打开远程 SFTP 子系统。
    pub async fn open_sftp(
        &mut self,
        session_id: SessionId,
    ) -> Result<RemoteSftp, BackendExecutionError> {
        let mut channel = self.open_session_channel("open sftp session").await?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|error| channel_error("request sftp", error))?;
        wait_channel_request(&mut channel, "request sftp").await?;

        let session = SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| sftp_error("open", error))?;

        Ok(RemoteSftp {
            session_id,
            session,
        })
    }
}

pub(super) fn sftp_entry_from_parts(
    remote_dir: &str,
    name: String,
    metadata: FileAttributes,
) -> SftpEntry {
    SftpEntry {
        remote_path: join_remote_path(remote_dir, &name),
        name,
        kind: sftp_entry_kind(metadata.file_type()),
        size: metadata.size,
        modified_at_unix_secs: metadata.mtime.map(u64::from),
        permissions: metadata.permissions,
    }
}

fn sftp_entry_kind(file_type: RusshSftpFileType) -> SftpEntryKind {
    match file_type {
        RusshSftpFileType::Dir => SftpEntryKind::Directory,
        RusshSftpFileType::File => SftpEntryKind::File,
        RusshSftpFileType::Symlink => SftpEntryKind::Symlink,
        RusshSftpFileType::Other => SftpEntryKind::Other,
    }
}

pub(super) fn join_remote_path(remote_dir: &str, name: &str) -> String {
    if remote_dir == "/" {
        format!("/{name}")
    } else {
        format!("{}/{}", remote_dir.trim_end_matches('/'), name)
    }
}

pub(super) fn parent_remote_dir(remote_path: &str) -> String {
    let path = remote_path.trim_end_matches('/');
    match path.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(index) => path[..index].to_owned(),
    }
}

fn transfer_events(
    session_id: SessionId,
    transfer_id: crate::model::TransferId,
    transferred_bytes: u64,
) -> Vec<BackendEvent> {
    vec![
        BackendEvent::TransferProgress {
            session_id,
            transfer_id,
            transferred_bytes: 0,
            status: TransferStatus::Running,
        },
        BackendEvent::TransferProgress {
            session_id,
            transfer_id,
            transferred_bytes,
            status: TransferStatus::Completed,
        },
    ]
}

fn sftp_error(operation: &str, error: impl std::fmt::Display) -> BackendExecutionError {
    BackendExecutionError::SftpFailed {
        operation: operation.to_owned(),
        reason: error.to_string(),
    }
}

fn sftp_io_error(operation: &str, error: std::io::Error) -> BackendExecutionError {
    BackendExecutionError::SftpFailed {
        operation: operation.to_owned(),
        reason: error.to_string(),
    }
}
