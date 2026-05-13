//! SSH 端口转发和隧道运行时。

use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use russh::Channel;
use russh::client;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{Duration, timeout};

use crate::backend::{BackendEvent, BackendExecutionError, TunnelStartRequest};
use crate::model::{SessionId, TunnelKind, TunnelStatus};

use super::super::{RusshConnection, SshClientHandler};

mod socks5;
use socks5::{read_socks5_target, write_socks5_success};

/// 运行中的 SSH 隧道句柄。
pub struct RemoteTunnel {
    rule_name: String,
    running: Arc<AtomicBool>,
    bind_host: String,
    bind_port: u16,
}

impl RemoteTunnel {
    /// 返回关联的隧道规则名称。
    pub fn rule_name(&self) -> &str {
        &self.rule_name
    }

    /// 返回本地或远端监听地址。
    pub fn bind_endpoint(&self) -> String {
        format!("{}:{}", self.bind_host, self.bind_port)
    }

    /// 请求隧道循环停止。已建立的连接会自然结束。
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

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
            while running_loop.load(Ordering::SeqCst) {
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
            tunnel(rule.name, running, rule.bind_host, rule.bind_port),
            vec![BackendEvent::TunnelStatusChanged {
                session_id,
                rule_name,
                status: TunnelStatus::Running,
            }],
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
            while running_loop.load(Ordering::SeqCst) {
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
            tunnel(rule.name, running, rule.bind_host, rule.bind_port),
            vec![BackendEvent::TunnelStatusChanged {
                session_id,
                rule_name,
                status: TunnelStatus::Running,
            }],
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
            while running_loop.load(Ordering::SeqCst) {
                let channel =
                    match timeout(Duration::from_millis(250), forwarded_channels.recv()).await {
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
            tunnel(rule.name, running, rule.bind_host, rule.bind_port),
            vec![BackendEvent::TunnelStatusChanged {
                session_id,
                rule_name,
                status: TunnelStatus::Running,
            }],
        ))
    }
}

fn tunnel(
    rule_name: String,
    running: Arc<AtomicBool>,
    bind_host: String,
    bind_port: u16,
) -> RemoteTunnel {
    RemoteTunnel {
        rule_name,
        running,
        bind_host,
        bind_port,
    }
}

async fn bind_tcp_listener(
    bind_host: &str,
    bind_port: u16,
    rule_name: &str,
) -> Result<TcpListener, BackendExecutionError> {
    TcpListener::bind((bind_host, bind_port))
        .await
        .map_err(|error| BackendExecutionError::TunnelFailed {
            rule_name: rule_name.to_owned(),
            reason: error.to_string(),
        })
}

async fn accept_with_tick(
    listener: &TcpListener,
) -> Result<Option<(TcpStream, SocketAddr)>, std::io::Error> {
    match timeout(Duration::from_millis(250), listener.accept()).await {
        Ok(result) => result.map(Some),
        Err(_) => Ok(None),
    }
}

async fn pipe_direct_tcpip(
    handle: &mut client::Handle<SshClientHandler>,
    mut socket: TcpStream,
    originator: SocketAddr,
    target_host: String,
    target_port: u16,
) -> Result<(), BackendExecutionError> {
    let channel = handle
        .channel_open_direct_tcpip(
            target_host,
            u32::from(target_port),
            originator.ip().to_string(),
            u32::from(originator.port()),
        )
        .await
        .map_err(|error| BackendExecutionError::ChannelFailed {
            operation: "direct tcpip".to_owned(),
            reason: error.to_string(),
        })?;
    let mut stream = channel.into_stream();
    copy_bidirectional(&mut socket, &mut stream)
        .await
        .map_err(|error| BackendExecutionError::TunnelFailed {
            rule_name: "direct-tcpip".to_owned(),
            reason: error.to_string(),
        })?;
    Ok(())
}

async fn pipe_forwarded_tcpip(
    channel: Channel<client::Msg>,
    target_host: String,
    target_port: u16,
) -> Result<(), BackendExecutionError> {
    let mut socket = TcpStream::connect((target_host.as_str(), target_port))
        .await
        .map_err(|error| BackendExecutionError::TunnelFailed {
            rule_name: "remote-forward".to_owned(),
            reason: error.to_string(),
        })?;
    let mut stream = channel.into_stream();
    copy_bidirectional(&mut stream, &mut socket)
        .await
        .map_err(|error| BackendExecutionError::TunnelFailed {
            rule_name: "remote-forward".to_owned(),
            reason: error.to_string(),
        })?;
    Ok(())
}

async fn serve_socks5_connection(
    handle: &mut client::Handle<SshClientHandler>,
    mut socket: TcpStream,
    originator: SocketAddr,
) -> Result<(), BackendExecutionError> {
    let target = read_socks5_target(&mut socket).await.map_err(|error| {
        BackendExecutionError::TunnelFailed {
            rule_name: "dynamic-socks5".to_owned(),
            reason: error,
        }
    })?;
    let channel = handle
        .channel_open_direct_tcpip(
            target.host.clone(),
            u32::from(target.port),
            originator.ip().to_string(),
            u32::from(originator.port()),
        )
        .await
        .map_err(|error| BackendExecutionError::ChannelFailed {
            operation: "dynamic socks5".to_owned(),
            reason: error.to_string(),
        })?;

    write_socks5_success(&mut socket).await.map_err(|error| {
        BackendExecutionError::TunnelFailed {
            rule_name: "dynamic-socks5".to_owned(),
            reason: error.to_string(),
        }
    })?;

    let mut stream = channel.into_stream();
    copy_bidirectional(&mut socket, &mut stream)
        .await
        .map_err(|error| BackendExecutionError::TunnelFailed {
            rule_name: "dynamic-socks5".to_owned(),
            reason: error.to_string(),
        })?;
    Ok(())
}

async fn copy_bidirectional<A, B>(left: &mut A, right: &mut B) -> std::io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let _ = tokio::io::copy_bidirectional(left, right).await?;
    Ok(())
}

fn tunnel_error(rule_name: &str, error: russh::Error) -> BackendExecutionError {
    BackendExecutionError::TunnelFailed {
        rule_name: rule_name.to_owned(),
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_tunnel_reports_endpoint_and_can_stop() {
        let running = Arc::new(AtomicBool::new(true));
        let tunnel = tunnel(
            "proxy".to_owned(),
            running.clone(),
            "127.0.0.1".to_owned(),
            1080,
        );

        assert_eq!(tunnel.rule_name(), "proxy");
        assert_eq!(tunnel.bind_endpoint(), "127.0.0.1:1080");
        tunnel.stop();
        assert!(!running.load(Ordering::SeqCst));
    }
}
