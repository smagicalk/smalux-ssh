//! 隧道 TCP 监听、转发和 SOCKS5 连接处理。

use std::net::SocketAddr;

use russh::Channel;
use russh::client;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

use crate::backend::{BackendExecutionError, ssh::SshClientHandler};
use smagical_ssh_client_core::{
    DIRECT_TCPIP_OPERATION, DIRECT_TCPIP_RULE_NAME, DYNAMIC_SOCKS5_OPERATION,
    DYNAMIC_SOCKS5_RULE_NAME, REMOTE_FORWARD_RULE_NAME, TUNNEL_ACCEPT_TICK, channel_error,
    copy_bidirectional, tunnel_io_error, tunnel_reason_error,
};

use super::socks5::{read_socks5_target, write_socks5_success};

pub(super) async fn bind_tcp_listener(
    bind_host: &str,
    bind_port: u16,
    rule_name: &str,
) -> Result<TcpListener, BackendExecutionError> {
    TcpListener::bind((bind_host, bind_port))
        .await
        .map_err(|error| tunnel_io_error(rule_name, error))
}

pub(super) async fn accept_with_tick(
    listener: &TcpListener,
) -> Result<Option<(TcpStream, SocketAddr)>, std::io::Error> {
    match timeout(TUNNEL_ACCEPT_TICK, listener.accept()).await {
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
        .map_err(|error| channel_error(DIRECT_TCPIP_OPERATION, error))?;
    let mut stream = channel.into_stream();
    copy_bidirectional(&mut socket, &mut stream)
        .await
        .map_err(|error| tunnel_io_error(DIRECT_TCPIP_RULE_NAME, error))?;
    Ok(())
}

pub(super) async fn pipe_forwarded_tcpip(
    channel: Channel<client::Msg>,
    target_host: String,
    target_port: u16,
) -> Result<(), BackendExecutionError> {
    let mut socket = TcpStream::connect((target_host.as_str(), target_port))
        .await
        .map_err(|error| tunnel_io_error(REMOTE_FORWARD_RULE_NAME, error))?;
    let mut stream = channel.into_stream();
    copy_bidirectional(&mut stream, &mut socket)
        .await
        .map_err(|error| tunnel_io_error(REMOTE_FORWARD_RULE_NAME, error))?;
    Ok(())
}

pub(super) async fn serve_socks5_connection(
    handle: &mut client::Handle<SshClientHandler>,
    mut socket: TcpStream,
    originator: SocketAddr,
) -> Result<(), BackendExecutionError> {
    let target = read_socks5_target(&mut socket)
        .await
        .map_err(|error| tunnel_reason_error(DYNAMIC_SOCKS5_RULE_NAME, error))?;
    let channel = handle
        .channel_open_direct_tcpip(
            target.host.clone(),
            u32::from(target.port),
            originator.ip().to_string(),
            u32::from(originator.port()),
        )
        .await
        .map_err(|error| channel_error(DYNAMIC_SOCKS5_OPERATION, error))?;

    write_socks5_success(&mut socket)
        .await
        .map_err(|error| tunnel_io_error(DYNAMIC_SOCKS5_RULE_NAME, error))?;

    let mut stream = channel.into_stream();
    copy_bidirectional(&mut socket, &mut stream)
        .await
        .map_err(|error| tunnel_io_error(DYNAMIC_SOCKS5_RULE_NAME, error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::backend::BackendExecutionError;

    use super::{accept_with_tick, bind_tcp_listener};

    #[tokio::test]
    async fn bind_tcp_listener_reports_rule_name_on_failure() {
        let listener = bind_tcp_listener("127.0.0.1", 0, "primary").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let error = bind_tcp_listener("127.0.0.1", port, "duplicate")
            .await
            .unwrap_err();

        match error {
            BackendExecutionError::TunnelFailed { rule_name, reason } => {
                assert_eq!(rule_name, "duplicate");
                assert!(!reason.is_empty());
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn accept_with_tick_returns_none_without_connection() {
        let listener = bind_tcp_listener("127.0.0.1", 0, "idle").await.unwrap();

        let accepted = accept_with_tick(&listener).await.unwrap();

        assert!(accepted.is_none());
    }
}
