//! SSH SFTP 子系统会话。

use russh_sftp::client::SftpSession;
use smagical_ssh_client_core::{
    CLOSE_SFTP_OPERATION, OPEN_SFTP_OPERATION, OPEN_SFTP_SESSION_OPERATION, REQUEST_SFTP_OPERATION,
    SFTP_SUBSYSTEM_NAME, channel_error, sftp_error,
};

use crate::backend::{BackendEvent, BackendExecutionError, SftpRequest};
use crate::model::SessionId;

use super::super::RusshConnection;
use super::{open_session_channel, wait_channel_request};

#[path = "sftp/ops.rs"]
mod ops;
#[path = "sftp/transfer.rs"]
mod transfer;

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
                self.remove_file_and_refresh_parent(remote_path).await
            }
            SftpRequest::CreateDir { remote_path } => {
                self.create_dir_and_refresh_parent(remote_path).await
            }
        }
    }

    /// 主动关闭 SFTP 子系统。
    pub async fn close(&self) -> Result<(), BackendExecutionError> {
        self.session
            .close()
            .await
            .map_err(|error| sftp_error(CLOSE_SFTP_OPERATION, error))
    }
}

impl RusshConnection {
    /// 打开远程 SFTP 子系统。
    pub async fn open_sftp(
        &mut self,
        session_id: SessionId,
    ) -> Result<RemoteSftp, BackendExecutionError> {
        let mut channel = open_session_channel(self, OPEN_SFTP_SESSION_OPERATION).await?;
        channel
            .request_subsystem(true, SFTP_SUBSYSTEM_NAME)
            .await
            .map_err(|error| channel_error(REQUEST_SFTP_OPERATION, error))?;
        wait_channel_request(&mut channel, REQUEST_SFTP_OPERATION).await?;

        let session = SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| sftp_error(OPEN_SFTP_OPERATION, error))?;

        Ok(RemoteSftp {
            session_id,
            session,
        })
    }
}
