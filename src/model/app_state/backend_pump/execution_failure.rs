//! 后端执行器错误到应用状态的归约。
//!
//! 这里集中处理真实 IO 执行失败后的状态一致性：
//! 1. 主机密钥拒绝时写入 Known Hosts 的未信任记录；
//! 2. 给当前会话或 SFTP 浏览器写入失败事件；
//! 3. 丢弃同一会话仍在队列中的依赖命令；
//! 4. 把最后错误写入 UI 状态，供通知栏或状态栏展示。

use crate::backend::{BackendEvent, BackendExecutionError};
use crate::model::SessionId;

use super::super::{AppState, AppUpdateOutcome};
use super::pending::{
    discard_pending_commands_for_failed_session, discard_pending_sftp_writes_for_failed_session,
};
use super::transfers::{FailedTransfer, transfer_failed_event};

pub(super) fn handle_backend_execution_error(
    state: &mut AppState,
    outcome: &mut AppUpdateOutcome,
    session_id: SessionId,
    failed_transfer: Option<FailedTransfer>,
    error: BackendExecutionError,
) {
    let reason = error.to_string();
    let is_sftp_operation_failure = sftp_operation_failed(&error);
    // HostKeyRejected 是连接失败的一种，但它还需要更新 Known Hosts 面板。
    outcome.state_changed |= state.record_rejected_host_key(&error);

    apply_backend_events(
        state,
        outcome,
        failed_backend_events(
            session_id,
            reason.clone(),
            failed_transfer,
            is_sftp_operation_failure,
        ),
    );

    // 失败后继续执行同一会话的排队命令通常没有意义。SFTP 写失败只清理写类命令，
    // 保留浏览类命令，避免一次上传失败把用户的目录刷新也一起吞掉。
    let discarded = if is_sftp_operation_failure {
        discard_pending_sftp_writes_for_failed_session(
            &mut state.backend_commands,
            session_id,
            &reason,
        )
    } else {
        discard_pending_commands_for_failed_session(
            &mut state.backend_commands,
            session_id,
            &reason,
        )
    };
    outcome.state_changed |= discarded.removed_count > 0;
    apply_backend_events(state, outcome, discarded.failure_events);

    outcome.state_changed |= state.ui.set_last_error(reason.clone());
    outcome.error = Some(reason);
}

/// 构造当前失败需要映射出的领域事件。
///
/// 传输失败需要额外更新进度列表；SFTP 操作失败则进入 SFTP 面板错误态；其他错误走会话
/// 通用失败事件。
fn failed_backend_events(
    session_id: SessionId,
    reason: String,
    transfer: Option<FailedTransfer>,
    sftp_operation_failed: bool,
) -> Vec<BackendEvent> {
    let mut events = Vec::new();
    if let Some(transfer) = transfer {
        events.push(transfer_failed_event(transfer, reason.clone()));
    }
    if sftp_operation_failed {
        events.push(BackendEvent::SftpFailed { session_id, reason });
    } else {
        events.push(BackendEvent::Failed { session_id, reason });
    }
    events
}

fn sftp_operation_failed(error: &BackendExecutionError) -> bool {
    matches!(error, BackendExecutionError::SftpFailed { .. })
}

/// 按既有事件归约路径应用失败事件，避免失败路径绕过正常状态更新规则。
fn apply_backend_events(
    state: &mut AppState,
    outcome: &mut AppUpdateOutcome,
    events: impl IntoIterator<Item = BackendEvent>,
) {
    for event in events {
        let event_outcome = state.apply_backend_event(event);
        outcome.state_changed |= event_outcome.state_changed;
        outcome.applied_backend_events += event_outcome.applied_backend_events;
    }
}
