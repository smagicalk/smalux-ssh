//! 后端泵失败路径中的待执行命令清理。

use crate::backend::{BackendCommandQueue, BackendEvent};
use crate::model::SessionId;

use super::transfers::{failed_transfer_for_command, is_sftp_write_command, transfer_failed_event};

#[derive(Debug, Default)]
pub(super) struct DiscardedPendingCommands {
    pub(super) removed_count: usize,
    pub(super) failure_events: Vec<BackendEvent>,
}

pub(super) fn discard_pending_commands_for_failed_session(
    commands: &mut BackendCommandQueue,
    session_id: SessionId,
    reason: &str,
) -> DiscardedPendingCommands {
    discard_matching_pending_commands(commands, reason, |command| {
        command.session_id() == session_id
    })
}

pub(super) fn discard_pending_sftp_writes_for_failed_session(
    commands: &mut BackendCommandQueue,
    session_id: SessionId,
    reason: &str,
) -> DiscardedPendingCommands {
    discard_matching_pending_commands(commands, reason, |command| {
        command.session_id() == session_id && is_sftp_write_command(command)
    })
}

fn discard_matching_pending_commands(
    commands: &mut BackendCommandQueue,
    reason: &str,
    should_discard: impl Fn(&crate::backend::BackendCommand) -> bool,
) -> DiscardedPendingCommands {
    let mut transfer_failures = Vec::new();
    let removed_count = commands.retain(|command| {
        if !should_discard(command) {
            return true;
        }

        if let Some(transfer) = failed_transfer_for_command(command) {
            transfer_failures.push(transfer);
        }
        false
    });
    let failure_events = transfer_failures
        .into_iter()
        .map(|transfer| transfer_failed_event(transfer, reason.to_owned()))
        .collect();

    DiscardedPendingCommands {
        removed_count,
        failure_events,
    }
}
