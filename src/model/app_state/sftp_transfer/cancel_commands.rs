//! SFTP 取消传输队列命令处理。

use crate::backend::{BackendCommand, BackendCommandQueue, SftpRequest};
use crate::model::{HostId, SessionId, TransferId};
use crate::session::SessionManager;

pub(super) fn remove_queued_sftp_transfer_command(
    commands: &mut BackendCommandQueue,
    task_session_id: SessionId,
    transfer_id: TransferId,
) -> usize {
    commands.retain(|command| !is_sftp_transfer_command(command, task_session_id, transfer_id))
}

pub(super) fn has_pending_sftp_browser_refresh(
    sessions: &SessionManager,
    commands: &BackendCommandQueue,
    host_id: HostId,
    current_session_id: SessionId,
) -> bool {
    commands.iter().any(|command| {
        let BackendCommand::Sftp {
            session_id,
            request,
        } = command
        else {
            return false;
        };

        request.refreshes_browser()
            && *session_id == current_session_id
            && session_matches_host(sessions, *session_id, host_id)
    })
}

fn is_sftp_transfer_command(
    command: &BackendCommand,
    task_session_id: SessionId,
    transfer_id: TransferId,
) -> bool {
    matches!(
        command,
        BackendCommand::Sftp {
            session_id,
            request:
                SftpRequest::Upload { id, .. } | SftpRequest::Download { id, .. },
            ..
        } if *session_id == task_session_id && *id == transfer_id
    )
}

fn session_matches_host(sessions: &SessionManager, session_id: SessionId, host_id: HostId) -> bool {
    sessions
        .tabs
        .iter()
        .any(|tab| tab.id == session_id && tab.host_id == Some(host_id))
}
