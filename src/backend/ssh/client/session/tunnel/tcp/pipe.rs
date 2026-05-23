//! 隧道 TCP 双向复制管道。

use std::net::SocketAddr;

use russh::Channel;
use russh::client;
use tokio::net::TcpStream;

use crate::backend::{BackendExecutionError, ssh::SshClientHandler};
use smagical_ssh_client_core::{
    DIRECT_TCPIP_OPERATION, DIRECT_TCPIP_RULE_NAME, REMOTE_FORWARD_RULE_NAME, channel_error,
    copy_bidirectional, tunnel_io_error,
};

pub(in crate::backend::ssh::client::session::tunnel) async fn pipe_direct_tcpip(
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

pub(in crate::backend::ssh::client::session::tunnel) async fn pipe_forwarded_tcpip(
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
