//! 后端执行器错误到应用状态的归约。

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
