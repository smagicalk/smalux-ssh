//! SSH executor 隧道运行逻辑。

use smagical_ssh_client_core::{
    START_TUNNEL_OPERATION, connected_session_error, tunnel_stopped_event,
};

use crate::backend::{BackendEvent, BackendExecutionError, TunnelStartRequest, TunnelStopRequest};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::RusshBackendExecutor;
use super::cache::{
    remove_tunnel_for_session_rule, replace_tunnel_stopping_previous,
    take_cached_session_subresources,
};

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    pub(super) fn start_tunnel(
        &mut self,
        session_id: SessionId,
        request: TunnelStartRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let connection = self
            .connections
            .remove(&session_id)
            .ok_or_else(|| connected_session_error(START_TUNNEL_OPERATION))?;
        let stale_subresources =
            take_cached_session_subresources(&mut self.shells, &mut self.sftps, session_id);
        self.close_stale_session_subresources(session_id, stale_subresources, "starting tunnel");
        let (tunnel, events) = self
            .runtime
            .block_on(connection.into_tunnel(session_id, request))?;
        replace_tunnel_stopping_previous(&mut self.tunnels, tunnel);
        Ok(events)
    }

    pub(super) fn stop_tunnel(
        &mut self,
        session_id: SessionId,
        request: TunnelStopRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let rule_name = request.rule_name;
        if let Some(tunnel) =
            remove_tunnel_for_session_rule(&mut self.tunnels, session_id, &rule_name)
        {
            tunnel.stop();
        }

        Ok(vec![tunnel_stopped_event(session_id, rule_name)])
    }
}
