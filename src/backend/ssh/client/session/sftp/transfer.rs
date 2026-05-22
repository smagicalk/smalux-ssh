//! SFTP 上传与下载传输。

use smagical_ssh_client_core::{
    DOWNLOAD_CLOSE_REMOTE_OPERATION, DOWNLOAD_FLUSH_LOCAL_OPERATION, DOWNLOAD_OPEN_LOCAL_OPERATION,
    DOWNLOAD_OPEN_REMOTE_OPERATION, DOWNLOAD_READ_REMOTE_OPERATION, DOWNLOAD_STAT_REMOTE_OPERATION,
    DOWNLOAD_WRITE_LOCAL_OPERATION, UPLOAD_CLOSE_REMOTE_OPERATION, UPLOAD_OPEN_LOCAL_OPERATION,
    UPLOAD_OPEN_REMOTE_OPERATION, UPLOAD_READ_LOCAL_OPERATION, UPLOAD_STAT_LOCAL_OPERATION,
    UPLOAD_WRITE_REMOTE_OPERATION, copy_transfer_with_progress, parent_remote_dir, sftp_error,
    sftp_io_error, transfer_event,
};
use tokio::io::AsyncWriteExt;

use crate::backend::{BackendEvent, BackendExecutionError};
use crate::model::{TransferId, TransferStatus};

use super::RemoteSftp;

impl RemoteSftp {
    pub(super) async fn upload(
        &self,
        id: TransferId,
        local_path: String,
        remote_path: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let mut local_file = tokio::fs::File::open(&local_path)
            .await
            .map_err(|error| sftp_io_error(UPLOAD_OPEN_LOCAL_OPERATION, error))?;
        let total_bytes = local_file
            .metadata()
            .await
            .map_err(|error| sftp_io_error(UPLOAD_STAT_LOCAL_OPERATION, error))?
            .len();
        let mut remote_file = self
            .session
            .create(remote_path.clone())
            .await
            .map_err(|error| sftp_error(UPLOAD_OPEN_REMOTE_OPERATION, error))?;
        let mut events = vec![transfer_event(
            self.session_id,
            id,
            Some(total_bytes),
            0,
            TransferStatus::Running,
        )];
        let (transferred_bytes, progress_events) = copy_transfer_with_progress(
            self.session_id,
            id,
            Some(total_bytes),
            &mut local_file,
            &mut remote_file,
            UPLOAD_READ_LOCAL_OPERATION,
            UPLOAD_WRITE_REMOTE_OPERATION,
        )
        .await?;
        events.extend(progress_events);

        remote_file
            .shutdown()
            .await
            .map_err(|error| sftp_io_error(UPLOAD_CLOSE_REMOTE_OPERATION, error))?;

        events.push(transfer_event(
            self.session_id,
            id,
            Some(total_bytes),
            transferred_bytes,
            TransferStatus::Completed,
        ));
        events.extend(self.list_dir(parent_remote_dir(&remote_path)).await?);
        Ok(events)
    }

    pub(super) async fn download(
        &self,
        id: TransferId,
        remote_path: String,
        local_path: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let mut remote_file = self
            .session
            .open(remote_path)
            .await
            .map_err(|error| sftp_error(DOWNLOAD_OPEN_REMOTE_OPERATION, error))?;
        let total_bytes = remote_file
            .metadata()
            .await
            .map_err(|error| sftp_error(DOWNLOAD_STAT_REMOTE_OPERATION, error))?
            .size;
        let mut local_file = tokio::fs::File::create(&local_path)
            .await
            .map_err(|error| sftp_io_error(DOWNLOAD_OPEN_LOCAL_OPERATION, error))?;
        let mut events = vec![transfer_event(
            self.session_id,
            id,
            total_bytes,
            0,
            TransferStatus::Running,
        )];
        let (transferred_bytes, progress_events) = copy_transfer_with_progress(
            self.session_id,
            id,
            total_bytes,
            &mut remote_file,
            &mut local_file,
            DOWNLOAD_READ_REMOTE_OPERATION,
            DOWNLOAD_WRITE_LOCAL_OPERATION,
        )
        .await?;
        events.extend(progress_events);

        local_file
            .flush()
            .await
            .map_err(|error| sftp_io_error(DOWNLOAD_FLUSH_LOCAL_OPERATION, error))?;
        remote_file
            .shutdown()
            .await
            .map_err(|error| sftp_io_error(DOWNLOAD_CLOSE_REMOTE_OPERATION, error))?;

        events.push(transfer_event(
            self.session_id,
            id,
            total_bytes,
            transferred_bytes,
            TransferStatus::Completed,
        ));
        Ok(events)
    }
}
