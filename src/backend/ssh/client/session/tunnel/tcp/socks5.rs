//! 隧道 TCP SOCKS5 连接处理。

use std::net::SocketAddr;

use russh::client;
use tokio::net::TcpStream;

use crate::backend::{BackendExecutionError, ssh::SshClientHandler};
use smagical_ssh_client_core::{
    DYNAMIC_SOCKS5_OPERATION, DYNAMIC_SOCKS5_RULE_NAME, channel_error, copy_bidirectional,
    tunnel_io_error, tunnel_reason_error,
};

use super::super::socks5::{read_socks5_target, write_socks5_success};

pub(in crate::backend::ssh::client::session::tunnel) async fn serve_socks5_connection(
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
