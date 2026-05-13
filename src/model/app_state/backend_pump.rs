//! 后端命令队列执行泵。

use crate::backend::BackendExecutor;

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 将当前队列中的后端命令交给执行器，并把返回事件归约到状态。
    pub fn drain_backend_queue(&mut self, executor: &mut impl BackendExecutor) -> AppUpdateOutcome {
        let mut outcome = AppUpdateOutcome::default();

        while let Some(command) = self.backend_commands.pop_front() {
            let events = match executor.execute(command) {
                Ok(events) => events,
                Err(error) => {
                    outcome.error = Some(error.to_string());
                    break;
                }
            };

            outcome.executed_backend_commands += 1;
            for event in events {
                let event_outcome = self.apply_backend_event(event);
                outcome.state_changed |= event_outcome.state_changed;
                outcome.applied_backend_events += event_outcome.applied_backend_events;
            }
        }

        outcome
    }
}
