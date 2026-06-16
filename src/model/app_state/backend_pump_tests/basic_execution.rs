use super::*;

#[test]
fn backend_queue_pump_executes_commands_and_applies_events() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let session_id = state.sessions.tabs[0].id;
    let mut executor = ScriptedBackendExecutor::new();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::Connect,
        vec![BackendEvent::Connected { session_id }],
    ));
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::OpenShell,
        vec![BackendEvent::Output {
            session_id,
            line: "ready".to_owned(),
        }],
    ));

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 2);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert_eq!(
        executor.executed(),
        &[BackendCommandKind::Connect, BackendCommandKind::OpenShell]
    );
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
    assert_eq!(state.terminal.tabs[0].buffer, vec!["ready"]);
}

#[test]
fn backend_queue_pump_executes_disconnect_for_closed_tabs() {
    let mut state = CoreState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state
        .backend_commands
        .push(BackendCommand::Disconnect { session_id });
    let mut executor = ScriptedBackendExecutor::new();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::Disconnect,
        vec![BackendEvent::Disconnected { session_id }],
    ));

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 1);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert_eq!(executor.executed(), &[BackendCommandKind::Disconnect]);
}

#[test]
fn backend_worker_command_path_defers_execution_and_applies_result() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let session_id = state.sessions.tabs[0].id;

    let queued = state.next_backend_command_for_worker();

    assert!(!queued.changed());
    assert!(matches!(
        queued.worker_command.as_ref(),
        Some(BackendCommand::Connect { .. })
    ));
    assert_eq!(state.backend_commands.pending_count(), 1);

    let command = queued.worker_command.expect("worker command should exist");
    let applied = state
        .apply_backend_command_result(command, Ok(vec![BackendEvent::Connected { session_id }]));

    assert!(applied.changed());
    assert_eq!(applied.executed_backend_commands, 1);
    assert_eq!(applied.applied_backend_events, 1);
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
}
