//! SSH 端口转发和隧道运行时。

use std::sync::{Arc, atomic::AtomicBool};

use tokio::sync::Mutex as AsyncMutex;
use tokio::time::timeout;

use crate::backend::{BackendEvent, BackendExecutionError, TunnelStartRequest};
use crate::model::{SessionId, TunnelKind};
use smagical_ssh_client_core::{
    TUNNEL_ACCEPT_TICK, tunnel_error, tunnel_is_running, tunnel_running_event,
};

use super::super::RusshConnection;

mod handle;
mod socks5;
mod tcp;

pub use handle::RemoteTunnel;
use handle::remote_tunnel;
use tcp::{
    accept_with_tick, bind_tcp_listener, pipe_direct_tcpip, pipe_forwarded_tcpip,
    serve_socks5_connection,
};

impl RusshConnection {
    /// 消费当前连接并启动端口转发或动态隧道。
    pub async fn into_tunnel(
        self,
        session_id: SessionId,
        request: TunnelStartRequest,
    ) -> Result<(RemoteTunnel, Vec<BackendEvent>), BackendExecutionError> {
        match request.rule.kind {
            TunnelKind::Local => self.start_local_tunnel(session_id, request).await,
            TunnelKind::Dynamic => self.start_dynamic_tunnel(session_id, request).await,
            TunnelKind::Remote => self.start_remote_tunnel(session_id, request).await,
        }
    }

    async fn start_local_tunnel(
        self,
        session_id: SessionId,
        request: TunnelStartRequest,
    ) -> Result<(RemoteTunnel, Vec<BackendEvent>), BackendExecutionError> {
        let rule = request.rule;
        let listener = bind_tcp_listener(&rule.bind_host, rule.bind_port, &rule.name).await?;
        let running = Arc::new(AtomicBool::new(true));
        let handle = Arc::new(AsyncMutex::new(self.handle));
        let rule_name = rule.name.clone();
        let target_host = rule.target_host.clone();
        let target_port = rule.target_port;
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
                let host = target_host.clone();
                tokio::spawn(async move {
                    let mut handle = handle.lock().await;
                    if let Err(error) =
                        pipe_direct_tcpip(&mut handle, socket, originator, host, target_port).await
                    {
                        tracing::warn!("local tunnel connection failed: {error}");
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

    async fn start_dynamic_tunnel(
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

    async fn start_remote_tunnel(
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
