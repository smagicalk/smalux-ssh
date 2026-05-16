//! 隧道 TCP 监听、转发和 SOCKS5 连接处理。

use std::net::SocketAddr;

use russh::Channel;
use russh::client;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, timeout};

use crate::backend::{BackendExecutionError, ssh::SshClientHandler};

use super::socks5::{read_socks5_target, write_socks5_success};

pub(super) async fn bind_tcp_listener(
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

pub(super) async fn accept_with_tick(
    listener: &TcpListener,
) -> Result<Option<(TcpStream, SocketAddr)>, std::io::Error> {
    match timeout(Duration::from_millis(250), listener.accept()).await {
        Ok(result) => result.map(Some),
        Err(_) => Ok(None),
    }
}

pub(super) async fn pipe_direct_tcpip(
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

pub(super) async fn pipe_forwarded_tcpip(
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

pub(super) async fn serve_socks5_connection(
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
