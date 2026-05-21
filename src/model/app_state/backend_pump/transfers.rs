//! 后端泵中的 SFTP 传输识别和失败事件构造。

use crate::backend::{BackendCommand, BackendEvent, SftpRequest};
use crate::model::{SessionId, TransferId, TransferStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FailedTransfer {
    session_id: SessionId,
    transfer_id: TransferId,
}

pub(super) fn failed_transfer_for_command(command: &BackendCommand) -> Option<FailedTransfer> {
    let BackendCommand::Sftp {
        session_id,
        request,
    } = command
    else {
        return None;
    };

    let transfer_id = match request {
        SftpRequest::Upload { id, .. } | SftpRequest::Download { id, .. } => *id,
        _ => return None,
    };

    Some(FailedTransfer {
        session_id: *session_id,
        transfer_id,
    })
}

pub(super) fn is_sftp_write_command(command: &BackendCommand) -> bool {
    let BackendCommand::Sftp { request, .. } = command else {
        return false;
    };

    !matches!(request, SftpRequest::ListDir { .. })
}

pub(super) fn transfer_failed_event(transfer: FailedTransfer, reason: String) -> BackendEvent {
    BackendEvent::TransferProgress {
        session_id: transfer.session_id,
        transfer_id: transfer.transfer_id,
        total_bytes: None,
        transferred_bytes: 0,
        status: TransferStatus::Failed { reason },
    }
}
