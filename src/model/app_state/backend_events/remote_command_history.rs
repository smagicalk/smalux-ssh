//! 远程命令历史完成状态回写。

use crate::model::{SessionId, SessionTab};

use crate::core::CoreState;
use remote_command_history_finish::RemoteCommandHistoryFinish;
use remote_command_history_match::{RemoteCommandHistoryMatch, remote_command_history_match};

#[path = "remote_command_history_finish.rs"]
mod remote_command_history_finish;
#[path = "remote_command_history_match.rs"]
mod remote_command_history_match;

impl CoreState {
    pub(in crate::model::app_state) fn finish_remote_command_history(
        &mut self,
        session_id: SessionId,
        exit_code: Option<i32>,
    ) -> bool {
        let Some(match_key) = self
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .and_then(remote_command_history_match)
        else {
            return false;
        };

        self.finish_matching_remote_command_history(
            match_key,
            RemoteCommandHistoryFinish::BackendEvent { exit_code },
        )
    }

    pub(in crate::model::app_state) fn finish_remote_command_history_for_closed_tab(
        &mut self,
        tab: &SessionTab,
    ) -> bool {
        let Some(match_key) = remote_command_history_match(tab) else {
            return false;
        };

        self.finish_matching_remote_command_history(
            match_key,
            RemoteCommandHistoryFinish::ClosedTab,
        )
    }

    fn finish_matching_remote_command_history(
        &mut self,
        match_key: RemoteCommandHistoryMatch,
        finish: RemoteCommandHistoryFinish,
    ) -> bool {
        let history = if let Some(history_id) = match_key.history_id {
            self.storage
                .command_history
                .iter_mut()
                .find(|item| item.id == history_id)
        } else {
            self.storage.command_history.iter_mut().rev().find(|item| {
                item.host_id == Some(match_key.host_id) && item.command == match_key.command
            })
        };

        let Some(history) = history else { return false };

        finish.apply_to(history)
    }
}
