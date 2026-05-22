//! SSH session 的一次性远程命令执行。

use smagical_ssh_client_core::{
    EXEC_COMMAND_OPERATION, RUN_COMMAND_OPERATION, RUN_COMMAND_SESSION_OPERATION, channel_error,
    collect_command_message, command_exited_event, remote_command_started_event,
};

use crate::backend::{BackendEvent, BackendExecutionError, RemoteCommandRequest};
use crate::model::SessionId;

use super::super::RusshConnection;
use super::{open_session_channel, prepare_pty, wait_channel_request};

pub(super) async fn run_remote_command(
    connection: &mut RusshConnection,
    session_id: SessionId,
    request: &RemoteCommandRequest,
) -> Result<Vec<BackendEvent>, BackendExecutionError> {
    let mut channel = open_session_channel(connection, RUN_COMMAND_SESSION_OPERATION).await?;
    if let Some(pty) = &request.pty {
        prepare_pty(&mut channel, pty, RUN_COMMAND_OPERATION).await?;
    }

    channel
        .exec(true, request.command.clone())
        .await
        .map_err(|error| channel_error(EXEC_COMMAND_OPERATION, error))?;
    wait_channel_request(&mut channel, EXEC_COMMAND_OPERATION).await?;

    let mut events = vec![remote_command_started_event(
        session_id,
        request.command.clone(),
    )];
    let mut exit_code = None;

    while let Some(message) = channel.wait().await {
        if collect_command_message(session_id, message, &mut events, &mut exit_code)? {
            break;
        }
    }

    events.push(command_exited_event(session_id, exit_code));

    Ok(events)
}
