//! 后端事件归约和共享执行器入口。
//!
//! 负责把后端事件应用到会话和终端状态，以及从共享执行器泵出后台命令。

use crate::backend::{BackendEvent, apply_backend_event};
use crate::model::{CommandHistoryId, HostId, SessionId, SessionKind};

use super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn apply_backend_event(&mut self, event: BackendEvent) -> AppUpdateOutcome {
        if let BackendEvent::CommandExited {
            session_id,
            exit_code,
        } = &event
        {
            // 会话状态更新前仍能读取 RemoteCommand 元数据，用于精确回写历史退出码。
            self.update_remote_command_history_exit_code(*session_id, *exit_code);
        }

        let outcome = apply_backend_event(&mut self.sessions, &mut self.terminal, event);

        AppUpdateOutcome {
            state_changed: outcome.changed(),
            applied_backend_events: 1,
            ..AppUpdateOutcome::default()
        }
    }

    /// 使用当前共享执行器泵出已排队的后台命令。
    pub fn drain_backend_queue_with_executor(&mut self) -> AppUpdateOutcome {
        let backend_executor = self.backend_executor.clone();
        let mut executor = backend_executor
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        self.drain_backend_queue(&mut **executor)
    }

    fn update_remote_command_history_exit_code(
        &mut self,
        session_id: SessionId,
        exit_code: Option<i32>,
    ) -> bool {
        let Some(match_key) = self.sessions.tabs.iter().find_map(|tab| {
            if tab.id != session_id {
                return None;
            };

            let SessionKind::RemoteCommand {
                command,
                history_id,
            } = &tab.kind
            else {
                return None;
            };

            let host_id = tab.host_id?;
            Some(RemoteCommandHistoryMatch {
                host_id,
                command: command.clone(),
                history_id: *history_id,
            })
        }) else {
            return false;
        };

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

        history.exit_code = exit_code;
        true
    }
}

struct RemoteCommandHistoryMatch {
    host_id: HostId,
    command: String,
    history_id: Option<CommandHistoryId>,
}
