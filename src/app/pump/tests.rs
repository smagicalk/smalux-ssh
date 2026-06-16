use super::drain::enqueue_drain_commands;
use crate::backend::BackendCommand;
use crate::core::CoreState;
use crate::model::{DEFAULT_LOCAL_TERMINAL_TITLE, SessionStatus};
use uuid::Uuid;

fn session_id() -> crate::model::SessionId {
    crate::model::SessionId(Uuid::new_v4())
}

#[test]
fn pump_drain_queue_targets_only_connected_interactive_shells() {
    let mut state = CoreState::default();
    let local_id = session_id();
    let shell_id = session_id();
    let remote_command_id = session_id();

    state
        .sessions
        .open_local_shell_tab(local_id, DEFAULT_LOCAL_TERMINAL_TITLE);
    state
        .sessions
        .open_shell_tab(shell_id, crate::model::HostId(Uuid::new_v4()), "ssh");
    state.sessions.open_remote_command_tab(
        remote_command_id,
        crate::model::HostId(Uuid::new_v4()),
        "uptime",
        None,
    );
    assert!(
        state
            .sessions
            .set_status(shell_id, SessionStatus::Connected)
    );
    assert!(
        state
            .sessions
            .set_status(remote_command_id, SessionStatus::RunningCommand)
    );

    let session_ids = state.sessions.interactive_shell_tab_ids();
    enqueue_drain_commands(&mut state, session_ids);

    assert_eq!(
        state.backend_commands.drain(),
        vec![
            BackendCommand::DrainSessionOutput {
                session_id: local_id
            },
            BackendCommand::DrainSessionOutput {
                session_id: shell_id
            }
        ]
    );
}

#[test]
fn pump_drain_queue_deduplicates_pending_session_drains() {
    let mut state = CoreState::default();
    let shell_id = session_id();

    state
        .sessions
        .open_shell_tab(shell_id, crate::model::HostId(Uuid::new_v4()), "ssh");
    assert!(
        state
            .sessions
            .set_status(shell_id, SessionStatus::Connected)
    );

    let session_ids = state.sessions.interactive_shell_tab_ids();
    enqueue_drain_commands(&mut state, session_ids.clone());
    enqueue_drain_commands(&mut state, session_ids);

    assert_eq!(
        state.backend_commands.drain(),
        vec![BackendCommand::DrainSessionOutput {
            session_id: shell_id
        }]
    );
}
