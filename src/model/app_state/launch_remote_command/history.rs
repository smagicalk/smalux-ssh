//! 远程命令历史调度。

use uuid::Uuid;

use crate::core::CoreState;
use crate::model::{CommandHistoryId, CommandHistoryItem, HostId};

use super::super::AppUpdateOutcome;
use super::super::launch::unix_now_secs;

impl CoreState {
    /// 重新执行一条带主机作用域的历史命令。
    pub(crate) fn run_command_history(&mut self, history_id: CommandHistoryId) -> AppUpdateOutcome {
        let Some(history) = self
            .storage
            .command_history
            .iter()
            .find(|item| item.id == history_id)
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到命令历史：{}", history_id.0)),
                ..AppUpdateOutcome::default()
            };
        };

        let Some(host_id) = history.host_id else {
            return AppUpdateOutcome {
                error: Some("命令历史缺少主机，无法直接重跑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let command = history.command.clone();

        self.run_remote_command(host_id, command, false)
    }

    pub(in crate::model::app_state) fn record_command_history(
        &mut self,
        host_id: HostId,
        command: String,
    ) -> CommandHistoryId {
        let history_id = CommandHistoryId(Uuid::new_v4());
        self.storage.add_command_history(CommandHistoryItem {
            id: history_id,
            host_id: Some(host_id),
            command,
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: unix_now_secs(),
            duration_ms: None,
        });
        history_id
    }
}
