//! 后端事件应用和共享执行器入口。

use crate::backend::{BackendEvent, apply_backend_event};

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(in crate::model::app_state) fn apply_backend_event(
        &mut self,
        event: BackendEvent,
    ) -> AppUpdateOutcome {
        match &event {
            BackendEvent::CommandExited {
                session_id,
                exit_code,
            } => {
                // 会话状态更新前仍能读取 RemoteCommand 元数据，用于精确回写命令历史结果。
                self.finish_remote_command_history(*session_id, *exit_code);
            }
            BackendEvent::Failed { session_id, .. } => {
                self.finish_remote_command_history(*session_id, None);
            }
            BackendEvent::Disconnected { session_id } => {
                self.finish_remote_command_history(*session_id, None);
            }
            _ => {}
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
}
