//! SSH executor 远程 shell 输出抽干逻辑。

use std::time::Duration;

use crate::backend::{BackendEvent, BackendExecutionError};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::super::RusshBackendExecutor;
use super::super::cache::remote_shell_events_require_cache_drop;

const REMOTE_SHELL_DRAIN_MAX_EVENTS: usize = 64;
const REMOTE_SHELL_DRAIN_POLL_TIMEOUT: Duration = Duration::from_millis(1);

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(in crate::backend::ssh::executor) fn drain_session_output(
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
}
