//! 后端泵中过期远程命令的状态收尾。

use crate::model::SessionId;

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn skip_stale_remote_command(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        AppUpdateOutcome {
            state_changed: self.finish_remote_command_history(session_id, None),
            ..AppUpdateOutcome::default()
        }
    }
}
