//! SSH executor 远程 shell 运行逻辑。

use std::time::Duration;

use smagical_ssh_client_core::{
    OPEN_SHELL_OPERATION, RUN_COMMAND_OPERATION, SEND_SHELL_INPUT_OPERATION,
    connected_session_error,
};

use crate::backend::{BackendEvent, BackendExecutionError, PtyRequest, RemoteCommandRequest};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::super::RemoteShell;
use super::RusshBackendExecutor;
use super::cache::{
    drop_cached_shell_after_failed_input, remote_shell_events_require_cache_drop,
    replace_cached_shell,
};

const REMOTE_SHELL_DRAIN_MAX_EVENTS: usize = 64;
const REMOTE_SHELL_DRAIN_POLL_TIMEOUT: Duration = Duration::from_millis(1);

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(super) fn open_shell(
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

    pub(super) fn send_shell_input(
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

    pub(super) fn drain_session_output(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let runtime = &self.runtime;
        let Some(shell) = self.shells.get_mut(&session_id) else {
            return Ok(Vec::new());
        };
        let events = runtime.block_on(shell.drain_ready_events(
            REMOTE_SHELL_DRAIN_MAX_EVENTS,
            REMOTE_SHELL_DRAIN_POLL_TIMEOUT,
        ));

        if remote_shell_events_require_cache_drop(session_id, &events) {
            self.shells.remove(&session_id);
        }

        Ok(events)
    }

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

    pub(super) fn close_detached_shell_input(
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
