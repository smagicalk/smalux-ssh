//! 远程命令历史完成状态回写。

use crate::model::{
    CommandHistoryId, CommandHistoryItem, HostId, SessionId, SessionKind, SessionTab,
};

use super::super::AppState;
use super::super::launch::unix_now_secs;

impl AppState {
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

struct RemoteCommandHistoryMatch {
    host_id: HostId,
    command: String,
    history_id: Option<CommandHistoryId>,
}

enum RemoteCommandHistoryFinish {
    BackendEvent { exit_code: Option<i32> },
    ClosedTab,
}

impl RemoteCommandHistoryFinish {
    fn apply_to(self, history: &mut CommandHistoryItem) -> bool {
        if command_history_is_finished(history) {
            return false;
        }

        let previous_exit_code = history.exit_code;
        let previous_duration_ms = history.duration_ms;

        if let RemoteCommandHistoryFinish::BackendEvent {
            exit_code: Some(exit_code),
        } = self
        {
            history.exit_code = Some(exit_code);
        }

        if history.duration_ms.is_none() {
            history.duration_ms = Some(command_duration_ms(history.started_at_unix_secs));
        }

        history.exit_code != previous_exit_code || history.duration_ms != previous_duration_ms
    }
}

fn command_history_is_finished(history: &CommandHistoryItem) -> bool {
    history.duration_ms.is_some()
}

fn remote_command_history_match(tab: &SessionTab) -> Option<RemoteCommandHistoryMatch> {
    let SessionKind::RemoteCommand {
        command,
        history_id,
    } = &tab.kind
    else {
        return None;
    };

    Some(RemoteCommandHistoryMatch {
        host_id: tab.host_id?,
        command: command.clone(),
        history_id: *history_id,
    })
}

fn command_duration_ms(started_at_unix_secs: u64) -> u64 {
    unix_now_secs()
        .saturating_sub(started_at_unix_secs)
        .saturating_mul(1_000)
}
