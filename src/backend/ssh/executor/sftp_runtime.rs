//! SSH executor SFTP 运行逻辑。

use smagical_ssh_client_core::{SFTP_OPERATION, connected_session_error};

use crate::backend::{BackendEvent, BackendExecutionError, SftpRequest};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::super::RemoteSftp;
use super::RusshBackendExecutor;
use super::cache::{drop_cached_sftp_after_failed_request, replace_cached_sftp};

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(super) fn sftp(
        &mut self,
        session_id: SessionId,
        request: SftpRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        if !self.sftps.contains_key(&session_id) {
            let runtime = &self.runtime;
            let connection = self
                .connections
                .get_mut(&session_id)
                .ok_or_else(|| connected_session_error(SFTP_OPERATION))?;
            let sftp = runtime.block_on(connection.open_sftp(session_id))?;
            let previous_sftp = replace_cached_sftp(&mut self.sftps, session_id, sftp);
            self.close_detached_sftp(session_id, previous_sftp, "opening sftp");
        }

        let sftp = self
            .sftps
            .get(&session_id)
            .ok_or_else(|| connected_session_error(SFTP_OPERATION))?;
        let result = self.runtime.block_on(sftp.execute(request));
        drop_cached_sftp_after_failed_request(&mut self.sftps, session_id, &result);
        result
    }

    pub(super) fn close_detached_sftp(
        &self,
        session_id: SessionId,
        sftp: Option<RemoteSftp>,
        operation: &'static str,
    ) {
        let Some(sftp) = sftp else {
            return;
        };

        if let Err(error) = self.runtime.block_on(sftp.close()) {
            tracing::warn!(
                session_id = %session_id.0,
                operation,
                error = %error,
                "failed to close detached SFTP session"
            );
        }
    }
}
