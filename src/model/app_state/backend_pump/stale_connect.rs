//! 后端泵中过期连接命令的状态收尾。

use crate::backend::BackendEvent;
use crate::model::SessionId;

use super::super::pending::discard_pending_commands_for_failed_session;
use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn skip_stale_connect_command(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        let reason = "连接命令已失效，后续启动命令未执行".to_owned();
        let event_outcome = self.apply_backend_event(BackendEvent::Failed {
            session_id,
            reason: reason.clone(),
        });
        let discarded = discard_pending_commands_for_failed_session(
            &mut self.backend_commands,
            session_id,
            &reason,
        );
        let mut outcome = AppUpdateOutcome {
            state_changed: event_outcome.state_changed || discarded.removed_count > 0,
            applied_backend_events: event_outcome.applied_backend_events,
            ..AppUpdateOutcome::default()
        };
        for event in discarded.failure_events {
            let event_outcome = self.apply_backend_event(event);
            outcome.state_changed |= event_outcome.state_changed;
            outcome.applied_backend_events += event_outcome.applied_backend_events;
        }
        outcome
    }
}
