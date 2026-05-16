//! 后端事件归约和共享执行器入口。
//!
//! 负责把后端事件应用到会话和终端状态，以及从共享执行器泵出后台命令。

use crate::backend::{BackendEvent, apply_backend_event};

use super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn apply_backend_event(&mut self, event: BackendEvent) -> AppUpdateOutcome {
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
