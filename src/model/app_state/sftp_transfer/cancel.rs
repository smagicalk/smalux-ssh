//! SFTP 传输取消调度。

use crate::backend::{BackendCommand, BackendCommandQueue, SftpRequest};
use crate::model::{HostId, SessionId, TransferDirection, TransferId, TransferTask};
use crate::session::SessionManager;

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 取消尚未交给后端执行器的 SFTP 传输。
    pub(in crate::model::app_state) fn cancel_sftp_transfer(
        &mut self,
        transfer_id: TransferId,
    ) -> AppUpdateOutcome {
        let task = match unique_transfer_task(&self.sessions.transfers, transfer_id) {
            TransferLookup::Found(task) => task,
            TransferLookup::Missing => {
                return AppUpdateOutcome {
                    error: Some(format!("找不到 SFTP 传输任务：{}", transfer_id.0)),
                    ..AppUpdateOutcome::default()
                };
            }
            TransferLookup::Ambiguous => {
                return AppUpdateOutcome {
                    error: Some(format!("SFTP 传输任务不唯一，无法取消：{}", transfer_id.0)),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        if !task.status.is_queued() {
            return AppUpdateOutcome {
                error: Some("只能取消尚未开始的 SFTP 传输".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let removed_commands = self
            .backend_commands
            .retain(|command| !is_sftp_transfer_command(command, task.session_id, transfer_id));
        if removed_commands == 0 {
            return AppUpdateOutcome {
                error: Some("SFTP 传输已经开始，无法从队列取消".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let has_pending_browser_refresh = has_pending_sftp_browser_refresh(
            &self.sessions,
            &self.backend_commands,
            task.host_id,
            task.session_id,
        );
        let transfer_cancelled = self
            .sessions
            .cancel_queued_transfer(task.session_id, transfer_id);
        let loading_cleared = clear_loading_for_cancelled_transfer(
            &mut self.sessions,
            &task,
            has_pending_browser_refresh,
        );

        AppUpdateOutcome {
            state_changed: transfer_cancelled || loading_cleared || removed_commands > 0,
            ..AppUpdateOutcome::default()
        }
    }
}

fn unique_transfer_task(tasks: &[TransferTask], transfer_id: TransferId) -> TransferLookup {
    let mut matches = tasks.iter().filter(|task| task.id == transfer_id);
    let Some(task) = matches.next() else {
        return TransferLookup::Missing;
    };
    if matches.next().is_some() {
        return TransferLookup::Ambiguous;
    }

    TransferLookup::Found(task.clone())
}

enum TransferLookup {
    Found(TransferTask),
    Missing,
    Ambiguous,
}

fn is_sftp_transfer_command(
    command: &BackendCommand,
    task_session_id: SessionId,
    transfer_id: TransferId,
) -> bool {
    matches!(
        command,
        BackendCommand::Sftp {
            session_id,
            request:
                SftpRequest::Upload { id, .. } | SftpRequest::Download { id, .. },
            ..
        } if *session_id == task_session_id && *id == transfer_id
    )
}

fn clear_loading_for_cancelled_transfer(
    sessions: &mut SessionManager,
    task: &TransferTask,
    has_pending_browser_refresh: bool,
) -> bool {
    if matches!(task.direction, TransferDirection::Upload) && !has_pending_browser_refresh {
        sessions.set_sftp_loading_for_session(task.session_id, false)
    } else {
        false
    }
}

fn has_pending_sftp_browser_refresh(
    sessions: &SessionManager,
    commands: &BackendCommandQueue,
    host_id: HostId,
    current_session_id: SessionId,
) -> bool {
    commands.iter().any(|command| {
        let BackendCommand::Sftp {
            session_id,
            request,
        } = command
        else {
            return false;
        };

        request.refreshes_browser()
            && *session_id == current_session_id
            && session_matches_host(sessions, *session_id, host_id)
    })
}

fn session_matches_host(sessions: &SessionManager, session_id: SessionId, host_id: HostId) -> bool {
    sessions
        .tabs
        .iter()
        .any(|tab| tab.id == session_id && tab.host_id == Some(host_id))
}
