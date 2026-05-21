//! 后端命令队列执行泵。

use crate::backend::BackendExecutor;

mod command_eligibility;
mod execution_failure;
mod host_keys;
mod pending;
mod stale_commands;
mod transfers;

use super::{AppState, AppUpdateOutcome};
use execution_failure::handle_backend_execution_error;
use transfers::failed_transfer_for_command;

impl AppState {
    /// 将当前队列中的后端命令交给执行器，并把返回事件归约到状态。
    pub fn drain_backend_queue(
        &mut self,
        executor: &mut (impl BackendExecutor + ?Sized),
    ) -> AppUpdateOutcome {
        let mut outcome = AppUpdateOutcome::default();

        while let Some(command) = self.backend_commands.pop_front() {
            if !self.can_execute_backend_command(&command) {
                let skip_outcome = self.skip_stale_backend_command(&command);
                outcome.state_changed |= skip_outcome.state_changed;
                outcome.applied_backend_events += skip_outcome.applied_backend_events;
                continue;
            }

            let session_id = command.session_id();
            let failed_transfer = failed_transfer_for_command(&command);
            let events = match executor.execute(command) {
                Ok(events) => events,
                Err(error) => {
                    handle_backend_execution_error(
                        self,
                        &mut outcome,
                        session_id,
                        failed_transfer,
                        error,
                    );
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
