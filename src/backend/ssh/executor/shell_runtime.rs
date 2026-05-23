//! SSH executor 远程 shell 运行逻辑。

mod input;
mod open;
mod output;

use smagical_ssh_client_core::{RUN_COMMAND_OPERATION, connected_session_error};

use crate::backend::{BackendEvent, BackendExecutionError, RemoteCommandRequest};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::RusshBackendExecutor;

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(super) fn run_command(
        &mut self,
        session_id: SessionId,
        request: RemoteCommandRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let runtime = &self.runtime;
        let connection = self
            .connections
            .get_mut(&session_id)
            .ok_or_else(|| connected_session_error(RUN_COMMAND_OPERATION))?;
        runtime.block_on(connection.run_command(session_id, &request))
    }
}
