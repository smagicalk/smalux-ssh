//! SSH executor 会话生命周期运行逻辑。

mod cleanup;

use smagical_ssh_client_core::disconnected_event;

use crate::backend::{BackendEvent, BackendExecutionError, ConnectionTarget};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::super::SshConnectionPlan;
use super::RusshBackendExecutor;
use super::cache::{stop_detached_tunnels, take_cached_session_runtime_resources};

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(super) fn connect(
        &mut self,
        session_id: SessionId,
        target: ConnectionTarget,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let plan = SshConnectionPlan::from_target(&target, &self.secret_store)?;
        let report = self
            .runtime
            .block_on(self.connector.connect(session_id, plan))?;
        let stale_runtime = take_cached_session_runtime_resources(
            &mut self.shells,
            &mut self.sftps,
            &mut self.connections,
            &mut self.tunnels,
            session_id,
        );
        self.close_stale_session_resources(session_id, stale_runtime.cached_resources);
        stop_detached_tunnels(session_id, stale_runtime.tunnels, "reconnecting");
        self.connections.insert(session_id, report.connection);
        Ok(report.events)
    }

    pub(super) fn disconnect(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let resources = take_cached_session_runtime_resources(
            &mut self.shells,
            &mut self.sftps,
            &mut self.connections,
            &mut self.tunnels,
            session_id,
        );
        self.close_disconnected_session_resources(session_id, resources.cached_resources);
        stop_detached_tunnels(session_id, resources.tunnels, "disconnecting");

        Ok(vec![disconnected_event(session_id)])
    }
}
