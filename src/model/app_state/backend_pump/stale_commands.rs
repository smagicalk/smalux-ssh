//! 后端泵中过期命令的状态收尾。

use crate::backend::{BackendCommand, BackendEvent, SftpRequest};

use super::super::{AppState, AppUpdateOutcome};
use super::pending::discard_pending_commands_for_failed_session;
use super::transfers::{failed_transfer_for_command, transfer_failed_event};

impl AppState {
    pub(super) fn skip_stale_backend_command(
        &mut self,
        command: &BackendCommand,
    ) -> AppUpdateOutcome {
        match command {
            BackendCommand::Connect { session_id, .. } => {
                let reason = "连接命令已失效，后续启动命令未执行".to_owned();
                let event_outcome = self.apply_backend_event(BackendEvent::Failed {
                    session_id: *session_id,
                    reason: reason.clone(),
                });
                let discarded = discard_pending_commands_for_failed_session(
                    &mut self.backend_commands,
                    *session_id,
                    &reason,
                );
                let mut outcome = AppUpdateOutcome {
                    state_changed: event_outcome.state_changed || discarded.removed_count > 0,
                    applied_backend_events: event_outcome.applied_backend_events,
                    ..AppUpdateOutcome::default()
                };
                for event in discarded.failure_events {
                    let event_outcome = self.apply_backend_event(event);
                    outcome.state_changed |= event_outcome.state_changed;
                    outcome.applied_backend_events += event_outcome.applied_backend_events;
                }
                outcome
            }
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
            BackendCommand::RunCommand { session_id, .. } => AppUpdateOutcome {
                state_changed: self.finish_remote_command_history(*session_id, None),
                ..AppUpdateOutcome::default()
            },
            _ => AppUpdateOutcome::default(),
        }
    }
}
