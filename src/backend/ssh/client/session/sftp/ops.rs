//! SFTP 目录与文件操作。

use smagical_ssh_client_core::{
    CREATE_DIR_OPERATION, LIST_DIR_OPERATION, REMOVE_FILE_OPERATION, parent_remote_dir,
    sftp_entries_event, sftp_entry_from_parts, sftp_error,
};

use crate::backend::{BackendEvent, BackendExecutionError};

use super::RemoteSftp;

impl RemoteSftp {
    pub(super) async fn list_dir(
        &self,
        remote_path: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let entries = self
            .session
            .read_dir(remote_path.clone())
            .await
            .map_err(|error| sftp_error(LIST_DIR_OPERATION, error))?
            .map(|entry| sftp_entry_from_parts(&remote_path, entry.file_name(), entry.metadata()))
            .collect();

        Ok(vec![sftp_entries_event(
            self.session_id,
            remote_path,
            entries,
        )])
    }

    pub(super) async fn remove_file_and_refresh_parent(
        &self,
        remote_path: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        self.session
            .remove_file(remote_path.clone())
            .await
            .map_err(|error| sftp_error(REMOVE_FILE_OPERATION, error))?;
        self.list_dir(parent_remote_dir(&remote_path)).await
    }

    pub(super) async fn create_dir_and_refresh_parent(
        &self,
        remote_path: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        self.session
            .create_dir(remote_path.clone())
            .await
            .map_err(|error| sftp_error(CREATE_DIR_OPERATION, error))?;
        self.list_dir(parent_remote_dir(&remote_path)).await
    }
}
