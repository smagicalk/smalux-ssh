//! 后端命令队列执行泵。

use crate::backend::{
    BackendCommand, BackendCommandQueue, BackendEvent, BackendExecutionError, BackendExecutor,
    SftpRequest,
};
use crate::model::{HostKeyVerification, KnownHostEntry, SessionId, TransferId, TransferStatus};

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 将当前队列中的后端命令交给执行器，并把返回事件归约到状态。
    pub fn drain_backend_queue(
        &mut self,
        executor: &mut (impl BackendExecutor + ?Sized),
    ) -> AppUpdateOutcome {
        let mut outcome = AppUpdateOutcome::default();

        while let Some(command) = self.backend_commands.pop_front() {
            if !self.can_execute_backend_command(&command) {
                let skip_outcome = self.skip_stale_backend_command(&command);
                outcome.state_changed |= skip_outcome.state_changed;
                outcome.applied_backend_events += skip_outcome.applied_backend_events;
                continue;
            }

            let session_id = command.session_id();
            let failed_transfer = failed_transfer_for_command(&command);
            let events = match executor.execute(command) {
                Ok(events) => events,
                Err(error) => {
                    let reason = error.to_string();
                    let is_sftp_operation_failure = sftp_operation_failed(&error);
                    outcome.state_changed |= self.record_rejected_host_key(&error);
                    let failure_events = failed_backend_events(
                        session_id,
                        reason.clone(),
                        failed_transfer,
                        is_sftp_operation_failure,
                    );
                    for event in failure_events {
                        let event_outcome = self.apply_backend_event(event);
                        outcome.state_changed |= event_outcome.state_changed;
                        outcome.applied_backend_events += event_outcome.applied_backend_events;
                    }
                    if !is_sftp_operation_failure {
                        let discarded = discard_pending_commands_for_failed_session(
                            &mut self.backend_commands,
                            session_id,
                            &reason,
                        );
                        outcome.state_changed |= discarded.removed_count > 0;
                        for event in discarded.failure_events {
                            let event_outcome = self.apply_backend_event(event);
                            outcome.state_changed |= event_outcome.state_changed;
                            outcome.applied_backend_events += event_outcome.applied_backend_events;
                        }
                    } else {
                        let discarded = discard_pending_sftp_writes_for_failed_session(
                            &mut self.backend_commands,
                            session_id,
                            &reason,
                        );
                        outcome.state_changed |= discarded.removed_count > 0;
                        for event in discarded.failure_events {
                            let event_outcome = self.apply_backend_event(event);
                            outcome.state_changed |= event_outcome.state_changed;
                            outcome.applied_backend_events += event_outcome.applied_backend_events;
                        }
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

impl AppState {
    fn can_execute_backend_command(&self, command: &BackendCommand) -> bool {
        match command {
            BackendCommand::SendShellInput { session_id, .. } => {
                self.sessions.can_send_interactive_shell_input(*session_id)
            }
            BackendCommand::DrainSessionOutput { session_id } => {
                self.sessions.can_drain_interactive_shell(*session_id)
            }
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::ListDir { .. },
            } => self.sessions.can_execute_sftp_browser_command(*session_id),
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::RemoveFile { .. } | SftpRequest::CreateDir { .. },
            } => self.sessions.can_execute_sftp_browser_command(*session_id),
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::Upload { .. } | SftpRequest::Download { .. },
            } => self.sessions.can_execute_sftp_transfer_command(*session_id),
            _ => true,
        }
    }

    fn skip_stale_backend_command(&mut self, command: &BackendCommand) -> AppUpdateOutcome {
        match command {
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::ListDir { .. },
            } => AppUpdateOutcome {
                state_changed: self
                    .sessions
                    .set_sftp_loading_for_session(*session_id, false),
                ..AppUpdateOutcome::default()
            },
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::RemoveFile { .. } | SftpRequest::CreateDir { .. },
            } => AppUpdateOutcome {
                state_changed: self
                    .sessions
                    .fail_sftp_browser_for_session(*session_id, "SFTP 会话已结束，操作未执行"),
                ..AppUpdateOutcome::default()
            },
            BackendCommand::Sftp { session_id, .. } => {
                let Some(transfer) = failed_transfer_for_command(command) else {
                    return AppUpdateOutcome::default();
                };
                let event_outcome = self.apply_backend_event(transfer_failed_event(
                    transfer,
                    "SFTP 会话已结束，传输未执行".to_owned(),
                ));
                let loading_cleared = self
                    .sessions
                    .set_sftp_loading_for_session(*session_id, false);
                AppUpdateOutcome {
                    state_changed: event_outcome.state_changed || loading_cleared,
                    applied_backend_events: event_outcome.applied_backend_events,
                    ..AppUpdateOutcome::default()
                }
            }
            _ => AppUpdateOutcome::default(),
        }
    }
}

impl AppState {
    fn record_rejected_host_key(&mut self, error: &BackendExecutionError) -> bool {
        let BackendExecutionError::HostKeyRejected {
            host,
            port,
            key_algorithm,
            fingerprint,
            verification,
        } = error
        else {
            return false;
        };
        if matches!(verification, HostKeyVerification::Mismatch { .. }) {
            return false;
        }

        self.storage.upsert_known_host(KnownHostEntry::untrusted(
            host.clone(),
            *port,
            key_algorithm.clone(),
            fingerprint.clone(),
        ));
        true
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
        events.push(transfer_failed_event(transfer, reason.clone()));
    }
    if sftp_operation_failed {
        events.push(BackendEvent::SftpFailed { session_id, reason });
    } else {
        events.push(BackendEvent::Failed { session_id, reason });
    }
    events
}

fn discard_pending_commands_for_failed_session(
    commands: &mut BackendCommandQueue,
    session_id: SessionId,
    reason: &str,
) -> DiscardedPendingCommands {
    let mut transfer_failures = Vec::new();
    let removed_count = commands.retain(|command| {
        if command.session_id() != session_id {
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

fn discard_pending_sftp_writes_for_failed_session(
    commands: &mut BackendCommandQueue,
    session_id: SessionId,
    reason: &str,
) -> DiscardedPendingCommands {
    let mut transfer_failures = Vec::new();
    let removed_count = commands.retain(|command| {
        if command.session_id() != session_id {
            return true;
        }

        if !is_sftp_write_command(command) {
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

fn transfer_failed_event(transfer: FailedTransfer, reason: String) -> BackendEvent {
    BackendEvent::TransferProgress {
        session_id: transfer.session_id,
        transfer_id: transfer.transfer_id,
        total_bytes: None,
        transferred_bytes: 0,
        status: TransferStatus::Failed { reason },
    }
}

fn sftp_operation_failed(error: &BackendExecutionError) -> bool {
    matches!(error, BackendExecutionError::SftpFailed { .. })
}

#[derive(Debug, Default)]
struct DiscardedPendingCommands {
    removed_count: usize,
    failure_events: Vec<BackendEvent>,
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

fn is_sftp_write_command(command: &BackendCommand) -> bool {
    let BackendCommand::Sftp { request, .. } = command else {
        return false;
    };

    !matches!(request, SftpRequest::ListDir { .. })
}
