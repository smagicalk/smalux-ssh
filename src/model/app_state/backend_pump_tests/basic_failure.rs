use super::*;

#[test]
fn backend_queue_pump_discards_failed_session_tail_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.as_deref().unwrap_or("").contains("不支持"));
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("不支持")
    ));
}

#[test]
fn backend_queue_pump_marks_failed_remote_command_history_finished() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RunRemoteCommand {
        host_id,
        command: "uptime".to_owned(),
        request_pty: false,
    });
    state.storage.command_history[0].started_at_unix_secs = 1;
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert_eq!(state.storage.command_history[0].exit_code, None);
    assert!(state.storage.command_history[0].duration_ms.is_some());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Failed { .. }
    ));
}

#[test]
fn backend_queue_pump_keeps_other_session_commands_after_terminal_error() {
    let mut state = AppState::default();
    let first = sample_host();
    let second = sample_host();
    let first_id = first.id;
    let second_id = second.id;
    state.storage.upsert_host(first);
    state.storage.upsert_host(second);
    state.apply(Message::OpenShell { host_id: first_id });
    state.apply(Message::OpenShell { host_id: second_id });
    let failed_session_id = state.sessions.tabs[0].id;
    let remaining_session_id = state.sessions.tabs[1].id;
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert_eq!(state.backend_commands.pending_count(), 2);
    assert!(
        state
            .backend_commands
            .iter()
            .all(|command| command.session_id() == remaining_session_id)
    );
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Failed { .. }
    ));
    assert!(matches!(
        state.sessions.tabs[1].status,
        SessionStatus::Connecting
    ));
    assert_ne!(failed_session_id, remaining_session_id);
}
