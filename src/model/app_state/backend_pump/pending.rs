//! 后端泵失败路径中的待执行命令清理。
//!
//! 队列中的命令有依赖顺序。例如连接失败后，后面的打开 shell、SFTP 浏览、传输命令都不应
//! 再执行。这个模块只负责从队列中移除这些命令，并为被移除的传输命令补发失败事件。

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
    // 会话级失败：同一 session 后续命令全部丢弃。
    discard_matching_pending_commands(commands, reason, |command| {
        command.session_id() == session_id
    })
}

pub(super) fn discard_pending_sftp_writes_for_failed_session(
    commands: &mut BackendCommandQueue,
    session_id: SessionId,
    reason: &str,
) -> DiscardedPendingCommands {
    // SFTP 写失败：只丢弃同一 session 的写类命令，目录刷新仍可继续排队。
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

        // retain 只返回移除数量；传输 UI 还需要知道具体哪个 transfer 失败。
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
