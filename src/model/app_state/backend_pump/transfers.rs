//! 后端泵中的 SFTP 传输识别和失败事件构造。
//!
//! 上传/下载在 UI 中有独立进度条，但后端队列里它们只是 SFTP 命令的一种。这个模块把
//! “命令是否对应一个传输”这件事封装起来，失败路径不需要理解所有 SFTP 请求细节。

use crate::backend::{BackendCommand, BackendEvent, SftpRequest};
use crate::model::{SessionId, TransferId, TransferStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FailedTransfer {
    /// 传输所属会话，用于定位 SFTP 面板和传输列表。
    session_id: SessionId,
    /// UI 传输列表中的稳定标识。
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
        // 只有上传/下载会展示传输进度；目录浏览、创建目录、删除文件没有 transfer id。
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

    // ListDir 是只读浏览命令，其余 SFTP 操作都可能改变远端或本地文件状态。
    !matches!(request, SftpRequest::ListDir { .. })
}

pub(super) fn transfer_failed_event(transfer: FailedTransfer, reason: String) -> BackendEvent {
    // 失败时不再知道完整字节数，直接把传输置为 Failed，UI 以 reason 展示错误。
    BackendEvent::TransferProgress {
        session_id: transfer.session_id,
        transfer_id: transfer.transfer_id,
        total_bytes: None,
        transferred_bytes: 0,
        status: TransferStatus::Failed { reason },
    }
}
