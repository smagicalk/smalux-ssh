//! 标签页关闭时的待执行后端命令清理。

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{SessionId, SessionKind, SessionTab, TransferId};

use super::super::AppState;

impl AppState {
    pub(super) fn remove_pending_backend_commands_for_session(
        &mut self,
        session_id: SessionId,
    ) -> PendingCloseCommandCleanup {
        let mut removed_connect = false;
        let mut removed_start_tunnel = false;
        let mut transfer_ids = Vec::new();
        let removed_count = self.backend_commands.retain(|command| {
            if command.session_id() != session_id {
                return true;
            }

            removed_connect |= matches!(command, BackendCommand::Connect { .. });
            removed_start_tunnel |= matches!(command, BackendCommand::StartTunnel { .. });
            if let Some(transfer_id) = sftp_transfer_id(command) {
                transfer_ids.push(transfer_id);
            }
            false
        });
        let cancelled_transfer_count = transfer_ids
            .into_iter()
            .filter(|transfer_id| {
                self.sessions
                    .cancel_queued_transfer(session_id, *transfer_id)
            })
            .count();

        PendingCloseCommandCleanup {
            removed_count,
            removed_connect,
            removed_start_tunnel,
            cancelled_transfer_count,
        }
    }
}

pub(super) fn should_disconnect_on_close(
    tab: &SessionTab,
    cleanup: &PendingCloseCommandCleanup,
) -> bool {
    let closed_before_connect = cleanup.removed_connect;
    let cancelled_connected_tunnel_launch = matches!(tab.kind, SessionKind::Tunnel { .. })
        && cleanup.removed_start_tunnel
        && !closed_before_connect;

    (cancelled_connected_tunnel_launch || !matches!(tab.kind, SessionKind::Tunnel { .. }))
        && !closed_before_connect
        && !tab.status.is_terminal()
}

fn sftp_transfer_id(command: &BackendCommand) -> Option<TransferId> {
    let BackendCommand::Sftp { request, .. } = command else {
        return None;
    };

    match request {
        SftpRequest::Upload { id, .. } | SftpRequest::Download { id, .. } => Some(*id),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PendingCloseCommandCleanup {
    removed_count: usize,
    removed_connect: bool,
    removed_start_tunnel: bool,
    cancelled_transfer_count: usize,
}

impl PendingCloseCommandCleanup {
    pub(super) fn changed(self) -> bool {
        self.removed_count > 0 || self.cancelled_transfer_count > 0
    }
}
