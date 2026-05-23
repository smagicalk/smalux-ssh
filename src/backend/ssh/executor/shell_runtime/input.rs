//! SSH executor 远程 shell 输入逻辑。

use smagical_ssh_client_core::{SEND_SHELL_INPUT_OPERATION, connected_session_error};

use crate::backend::{BackendEvent, BackendExecutionError};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::super::super::RemoteShell;
use super::super::RusshBackendExecutor;
use super::super::cache::drop_cached_shell_after_failed_input;

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(in crate::backend::ssh::executor) fn send_shell_input(
        &mut self,
        session_id: SessionId,
        input: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let runtime = &self.runtime;
        let shell = self
            .shells
            .get(&session_id)
            .ok_or_else(|| connected_session_error(SEND_SHELL_INPUT_OPERATION))?;
        let result = runtime.block_on(shell.send_input(input.as_bytes()));
        drop_cached_shell_after_failed_input(&mut self.shells, session_id, &result);
        result?;
        Ok(Vec::new())
    }

    pub(in crate::backend::ssh::executor) fn close_detached_shell_input(
        &self,
        session_id: SessionId,
        shell: Option<RemoteShell>,
        operation: &'static str,
    ) {
        let Some(shell) = shell else {
            return;
        };

        if let Err(error) = self.runtime.block_on(shell.close_input()) {
            tracing::warn!(
                session_id = %session_id.0,
                operation,
                error = %error,
                "failed to close detached remote shell input"
            );
        }
    }
}
