//! SSH 动态 SOCKS5 端口转发运行时。

use std::sync::{Arc, atomic::AtomicBool};

use tokio::sync::Mutex as AsyncMutex;

use crate::backend::{BackendEvent, BackendExecutionError, TunnelStartRequest};
use crate::model::SessionId;
use smagical_ssh_client_core::{tunnel_is_running, tunnel_running_event};

use crate::backend::ssh::client::RusshConnection;

use super::handle::RemoteTunnel;
use super::handle::remote_tunnel;
use super::tcp::{accept_with_tick, bind_tcp_listener, serve_socks5_connection};

impl RusshConnection {
    pub(super) async fn start_dynamic_tunnel(
        self,
        session_id: SessionId,
        request: TunnelStartRequest,
    ) -> Result<(RemoteTunnel, Vec<BackendEvent>), BackendExecutionError> {
        let rule = request.rule;
        let listener = bind_tcp_listener(&rule.bind_host, rule.bind_port, &rule.name).await?;
        let running = Arc::new(AtomicBool::new(true));
        let handle = Arc::new(AsyncMutex::new(self.handle));
        let rule_name = rule.name.clone();
        let running_loop = running.clone();

        tokio::spawn(async move {
            while tunnel_is_running(&running_loop) {
                let Ok(accepted) = accept_with_tick(&listener).await else {
                    break;
                };
                let Some((socket, originator)) = accepted else {
                    continue;
                };

                let handle = handle.clone();
                tokio::spawn(async move {
                    let mut handle = handle.lock().await;
                    if let Err(error) =
                        serve_socks5_connection(&mut handle, socket, originator).await
                    {
                        tracing::warn!("dynamic tunnel connection failed: {error}");
                    }
                });
            }
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
