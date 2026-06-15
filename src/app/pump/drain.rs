//! 后端输出泵的排队逻辑。

use crate::backend::BackendCommand;
use crate::core::CoreState;

pub(super) fn enqueue_drain_commands(
    state: &mut CoreState,
    session_ids: impl IntoIterator<Item = crate::model::SessionId>,
) {
    for session_id in session_ids {
        if has_pending_drain_command(state, session_id) {
            continue;
        }

        state
            .backend_commands
            .push(BackendCommand::DrainSessionOutput { session_id });
    }
}

fn has_pending_drain_command(state: &CoreState, session_id: crate::model::SessionId) -> bool {
    state.backend_commands.iter().any(|command| {
        matches!(
            command,
            BackendCommand::DrainSessionOutput {
                session_id: pending_session_id,
            } if *pending_session_id == session_id
        )
    })
}
