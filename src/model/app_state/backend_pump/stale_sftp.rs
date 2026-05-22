//! 后端泵中过期 SFTP 命令的状态收尾。

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::SessionId;

use super::super::transfers::{failed_transfer_for_command, transfer_failed_event};
use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn skip_stale_sftp_command(
        &mut self,
        session_id: SessionId,
        request: &SftpRequest,
        command: &BackendCommand,
    ) -> AppUpdateOutcome {
        match request {
            SftpRequest::ListDir { .. } => AppUpdateOutcome {
                state_changed: self
                    .sessions
                    .set_sftp_loading_for_session(session_id, false),
                ..AppUpdateOutcome::default()
            },
            SftpRequest::RemoveFile { .. } | SftpRequest::CreateDir { .. } => AppUpdateOutcome {
                state_changed: self
                    .sessions
                    .fail_sftp_browser_for_session(session_id, "SFTP 会话已结束，操作未执行"),
                ..AppUpdateOutcome::default()
            },
            _ => self.skip_stale_sftp_transfer_command(session_id, command),
        }
    }

    fn skip_stale_sftp_transfer_command(
        &mut self,
        session_id: SessionId,
        command: &BackendCommand,
    ) -> AppUpdateOutcome {
        let Some(transfer) = failed_transfer_for_command(command) else {
            return AppUpdateOutcome::default();
        };
        let event_outcome = self.apply_backend_event(transfer_failed_event(
            transfer,
            "SFTP 会话已结束，传输未执行".to_owned(),
        ));
        let loading_cleared = self
            .sessions
            .set_sftp_loading_for_session(session_id, false);
        AppUpdateOutcome {
            state_changed: event_outcome.state_changed || loading_cleared,
            applied_backend_events: event_outcome.applied_backend_events,
            ..AppUpdateOutcome::default()
        }
    }
}
