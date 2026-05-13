//! 后端命令队列执行泵。

use crate::backend::{BackendEvent, BackendExecutor};

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 将当前队列中的后端命令交给执行器，并把返回事件归约到状态。
    pub fn drain_backend_queue(
        &mut self,
        executor: &mut (impl BackendExecutor + ?Sized),
    ) -> AppUpdateOutcome {
        let mut outcome = AppUpdateOutcome::default();

        while let Some(command) = self.backend_commands.pop_front() {
            let session_id = command.session_id();
            let events = match executor.execute(command) {
                Ok(events) => events,
                Err(error) => {
                    let reason = error.to_string();
                    let event_outcome = self.apply_backend_event(BackendEvent::Failed {
                        session_id,
                        reason: reason.clone(),
                    });
                    outcome.state_changed |= event_outcome.state_changed;
                    outcome.state_changed |= self.ui.set_last_error(reason.clone());
                    outcome.applied_backend_events += event_outcome.applied_backend_events;
                    outcome.error = Some(reason);
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
