//! 后端泵中过期命令的状态收尾。

use crate::backend::BackendCommand;

use super::super::{AppState, AppUpdateOutcome};

#[path = "stale_connect.rs"]
mod stale_connect;
#[path = "stale_remote_command.rs"]
mod stale_remote_command;
#[path = "stale_sftp.rs"]
mod stale_sftp;

impl AppState {
    pub(super) fn skip_stale_backend_command(
        &mut self,
        command: &BackendCommand,
    ) -> AppUpdateOutcome {
        match command {
            BackendCommand::Connect { session_id, .. } => {
                self.skip_stale_connect_command(*session_id)
            }
            BackendCommand::Sftp {
                session_id,
                request,
            } => self.skip_stale_sftp_command(*session_id, request, command),
            BackendCommand::RunCommand { session_id, .. } => {
                self.skip_stale_remote_command(*session_id)
            }
            _ => AppUpdateOutcome::default(),
        }
    }
}
