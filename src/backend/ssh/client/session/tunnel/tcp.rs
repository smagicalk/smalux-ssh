//! 隧道 TCP 监听、转发和 SOCKS5 连接处理。

use std::net::SocketAddr;

use russh::Channel;
use russh::client;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, timeout};

use crate::backend::{BackendExecutionError, ssh::SshClientHandler};
use smagical_ssh_client_core::{channel_error, tunnel_io_error, tunnel_reason_error};

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
        .map_err(|error| channel_error("direct tcpip", error))?;
    let mut stream = channel.into_stream();
    copy_bidirectional(&mut socket, &mut stream)
        .await
        .map_err(|error| tunnel_io_error("direct-tcpip", error))?;
    Ok(())
}

pub(super) async fn pipe_forwarded_tcpip(
    channel: Channel<client::Msg>,
    target_host: String,
    target_port: u16,
) -> Result<(), BackendExecutionError> {
    let mut socket = TcpStream::connect((target_host.as_str(), target_port))
        .await
        .map_err(|error| tunnel_io_error("remote-forward", error))?;
    let mut stream = channel.into_stream();
    copy_bidirectional(&mut stream, &mut socket)
        .await
        .map_err(|error| tunnel_io_error("remote-forward", error))?;
    Ok(())
}

pub(super) async fn serve_socks5_connection(
    handle: &mut client::Handle<SshClientHandler>,
    mut socket: TcpStream,
    originator: SocketAddr,
) -> Result<(), BackendExecutionError> {
    let target = read_socks5_target(&mut socket)
        .await
        .map_err(|error| tunnel_reason_error("dynamic-socks5", error))?;
    let channel = handle
        .channel_open_direct_tcpip(
            target.host.clone(),
            u32::from(target.port),
            originator.ip().to_string(),
            u32::from(originator.port()),
        )
        .await
        .map_err(|error| channel_error("dynamic socks5", error))?;

    write_socks5_success(&mut socket)
        .await
        .map_err(|error| tunnel_io_error("dynamic-socks5", error))?;

    let mut stream = channel.into_stream();
    copy_bidirectional(&mut socket, &mut stream)
        .await
        .map_err(|error| tunnel_io_error("dynamic-socks5", error))?;
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

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use crate::backend::BackendExecutionError;

    use super::{accept_with_tick, bind_tcp_listener, copy_bidirectional};

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

    #[tokio::test]
    async fn copy_bidirectional_moves_bytes_in_both_directions() {
        let left_listener = bind_tcp_listener("127.0.0.1", 0, "left").await.unwrap();
        let right_listener = bind_tcp_listener("127.0.0.1", 0, "right").await.unwrap();
        let left_addr = left_listener.local_addr().unwrap();
        let right_addr = right_listener.local_addr().unwrap();

        let mut left_client = tokio::net::TcpStream::connect(left_addr).await.unwrap();
        let mut right_client = tokio::net::TcpStream::connect(right_addr).await.unwrap();
        let (mut left_server, _) = left_listener.accept().await.unwrap();
        let (mut right_server, _) = right_listener.accept().await.unwrap();

        let pipe = tokio::spawn(async move {
            copy_bidirectional(&mut left_server, &mut right_server)
                .await
                .unwrap();
        });

        left_client.write_all(b"left-to-right").await.unwrap();
        right_client.write_all(b"right-to-left").await.unwrap();
        let mut from_left = vec![0_u8; b"left-to-right".len()];
        let mut from_right = vec![0_u8; b"right-to-left".len()];
        right_client.read_exact(&mut from_left).await.unwrap();
        left_client.read_exact(&mut from_right).await.unwrap();

        assert_eq!(from_left, b"left-to-right");
        assert_eq!(from_right, b"right-to-left");

        drop(left_client);
        drop(right_client);
        pipe.await.unwrap();
    }
}
