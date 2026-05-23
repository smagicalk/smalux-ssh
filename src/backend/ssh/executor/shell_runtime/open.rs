//! SSH executor 远程 shell 打开逻辑。

use smagical_ssh_client_core::{OPEN_SHELL_OPERATION, connected_session_error};

use crate::backend::{BackendEvent, BackendExecutionError, PtyRequest};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::super::RusshBackendExecutor;
use super::super::cache::replace_cached_shell;

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(in crate::backend::ssh::executor) fn open_shell(
        &mut self,
        session_id: SessionId,
        pty: PtyRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let runtime = &self.runtime;
        let connection = self
            .connections
            .get_mut(&session_id)
            .ok_or_else(|| connected_session_error(OPEN_SHELL_OPERATION))?;
        let report = runtime.block_on(connection.open_shell(session_id, &pty))?;
        let previous_shell = replace_cached_shell(&mut self.shells, session_id, report.shell);
        self.close_detached_shell_input(session_id, previous_shell, "opening shell");
        Ok(report.events)
    }
}
