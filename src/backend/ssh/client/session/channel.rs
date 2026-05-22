//! SSH session channel 与 PTY 工具。

use russh::Channel;
use russh::client;
use smagical_ssh_client_core::{
    ChannelRequestStatus, channel_error, channel_request_ended_error,
    collect_channel_request_message, pty_columns, pty_rows,
};

use crate::backend::{BackendExecutionError, PtyRequest};

use super::super::RusshConnection;

pub(super) async fn open_session_channel(
    connection: &mut RusshConnection,
    operation: &str,
) -> Result<Channel<client::Msg>, BackendExecutionError> {
    connection
        .handle_mut()
        .channel_open_session()
        .await
        .map_err(|error| channel_error(operation, error))
}

pub(super) async fn prepare_pty(
    channel: &mut Channel<client::Msg>,
    pty: &PtyRequest,
    operation: &str,
) -> Result<(), BackendExecutionError> {
    for (name, value) in &pty.environment {
        channel
            .set_env(false, name.clone(), value.clone())
            .await
            .map_err(|error| channel_error(operation, error))?;
    }

    channel
        .request_pty(
            true,
            &pty.term,
            pty_columns(pty.size),
            pty_rows(pty.size),
            0,
            0,
            &[],
        )
        .await
        .map_err(|error| channel_error(operation, error))?;
    wait_channel_request(channel, operation).await
}

pub(super) async fn wait_channel_request(
    channel: &mut Channel<client::Msg>,
    operation: &str,
) -> Result<(), BackendExecutionError> {
    while let Some(message) = channel.wait().await {
        if collect_channel_request_message(operation, message)? == ChannelRequestStatus::Accepted {
            return Ok(());
        }
    }

    Err(channel_request_ended_error(operation))
}
