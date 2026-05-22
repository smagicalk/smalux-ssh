//! 远程命令历史完成状态应用。

use crate::model::CommandHistoryItem;

use super::super::super::launch::unix_now_secs;

pub(super) enum RemoteCommandHistoryFinish {
    BackendEvent { exit_code: Option<i32> },
    ClosedTab,
}

impl RemoteCommandHistoryFinish {
    pub(super) fn apply_to(self, history: &mut CommandHistoryItem) -> bool {
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

fn command_duration_ms(started_at_unix_secs: u64) -> u64 {
    unix_now_secs()
        .saturating_sub(started_at_unix_secs)
        .saturating_mul(1_000)
}
