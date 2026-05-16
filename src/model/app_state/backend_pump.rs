//! 后端命令队列执行泵。

use crate::backend::{
    BackendCommand, BackendEvent, BackendExecutionError, BackendExecutor, SftpRequest,
};
use crate::model::{SessionId, TransferId, TransferStatus};

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 将当前队列中的后端命令交给执行器，并把返回事件归约到状态。
    pub fn drain_backend_queue(
        &mut self,
        executor: &mut (impl BackendExecutor + ?Sized),
    ) -> AppUpdateOutcome {
        let mut outcome = AppUpdateOutcome::default();

        while let Some(command) = self.backend_commands.pop_front() {
            let session_id = command.session_id();
            let failed_transfer = failed_transfer_for_command(&command);
            let events = match executor.execute(command) {
                Ok(events) => events,
                Err(error) => {
                    let reason = error.to_string();
                    let failure_events = failed_backend_events(
                        session_id,
                        reason.clone(),
                        failed_transfer,
                        sftp_operation_failed(&error),
                    );
                    for event in failure_events {
                        let event_outcome = self.apply_backend_event(event);
                        outcome.state_changed |= event_outcome.state_changed;
                        outcome.applied_backend_events += event_outcome.applied_backend_events;
                    }
                    outcome.state_changed |= self.ui.set_last_error(reason.clone());
                    outcome.error = Some(reason);
                    break;
                }
            };

            outcome.executed_backend_commands += 1;
            for event in events {
                let event_outcome = self.apply_backend_event(event);
                outcome.state_changed |= event_outcome.state_changed;
                outcome.applied_backend_events += event_outcome.applied_backend_events;
            }
        }

        outcome
    }
}

fn failed_backend_events(
    session_id: SessionId,
    reason: String,
    transfer: Option<FailedTransfer>,
    sftp_operation_failed: bool,
) -> Vec<BackendEvent> {
    let mut events = Vec::new();
    if let Some(transfer) = transfer {
        events.push(BackendEvent::TransferProgress {
            session_id: transfer.session_id,
            transfer_id: transfer.transfer_id,
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Failed {
                reason: reason.clone(),
            },
        });
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FailedTransfer {
    session_id: SessionId,
    transfer_id: TransferId,
}

fn failed_transfer_for_command(command: &BackendCommand) -> Option<FailedTransfer> {
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
