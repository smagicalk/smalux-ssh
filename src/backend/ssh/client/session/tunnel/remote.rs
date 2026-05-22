//! SSH 远程端口转发运行时。

use std::sync::{Arc, atomic::AtomicBool};

use tokio::time::timeout;

use crate::backend::{BackendEvent, BackendExecutionError, TunnelStartRequest};
use crate::model::SessionId;
use smagical_ssh_client_core::{
    TUNNEL_ACCEPT_TICK, tunnel_error, tunnel_is_running, tunnel_running_event,
};

use crate::backend::ssh::client::RusshConnection;

use super::handle::RemoteTunnel;
use super::handle::remote_tunnel;
use super::tcp::pipe_forwarded_tcpip;

impl RusshConnection {
    pub(super) async fn start_remote_tunnel(
        mut self,
        session_id: SessionId,
        request: TunnelStartRequest,
    ) -> Result<(RemoteTunnel, Vec<BackendEvent>), BackendExecutionError> {
        let rule = request.rule;
        let rule_name = rule.name.clone();
        let target_host = rule.target_host.clone();
        let target_port = rule.target_port;
        let running = Arc::new(AtomicBool::new(true));
        let mut forwarded_channels =
            self.subscribe_forwarded_channels(&rule.bind_host, rule.bind_port);

        self.handle_mut()
            .tcpip_forward(rule.bind_host.clone(), u32::from(rule.bind_port))
            .await
            .map_err(|error| tunnel_error(&rule.name, error))?;

        let running_loop = running.clone();
        let handle = self.handle;
        let bind_host_for_cancel = rule.bind_host.clone();
        let bind_port_for_cancel = rule.bind_port;
        tokio::spawn(async move {
            while tunnel_is_running(&running_loop) {
                let channel = match timeout(TUNNEL_ACCEPT_TICK, forwarded_channels.recv()).await {
                    Ok(Some(channel)) => channel,
                    Ok(None) => break,
                    Err(_) => continue,
                };
                let host = target_host.clone();
                tokio::spawn(async move {
                    if let Err(error) = pipe_forwarded_tcpip(channel, host, target_port).await {
                        tracing::warn!("remote tunnel connection failed: {error}");
                    }
                });
            }
            let _ = handle
                .cancel_tcpip_forward(bind_host_for_cancel, u32::from(bind_port_for_cancel))
                .await;
        });

        Ok((
            remote_tunnel(
                session_id,
                rule.name,
                running,
                rule.bind_host,
                rule.bind_port,
            ),
            vec![tunnel_running_event(session_id, rule_name)],
        ))
    }
}
