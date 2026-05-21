use super::*;

#[test]
fn backend_queue_pump_executes_commands_and_applies_events() {
    let mut state = AppState::default();
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
    let mut state = AppState::default();
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
fn backend_queue_pump_records_unknown_host_key_candidate() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let mut executor = RejectingHostKeyExecutor::new(HostKeyVerification::Unknown);

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.known_host_count(), 1);
    assert_eq!(
        state.storage.known_hosts[0],
        KnownHostEntry::untrusted("example.com", 22, KeyAlgorithm::Ed25519, "SHA256:new")
    );
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("主机密钥未被信任")
    ));
}

#[test]
fn backend_queue_pump_does_not_overwrite_trusted_host_on_mismatch() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.storage.upsert_known_host(KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: "SHA256:old".to_owned(),
        trusted: true,
    });
    state.apply(Message::OpenShell { host_id });
    let mut executor = RejectingHostKeyExecutor::new(HostKeyVerification::Mismatch {
        expected: "SHA256:old".to_owned(),
        actual: "SHA256:new".to_owned(),
    });

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(state.storage.known_host_count(), 1);
    assert_eq!(state.storage.known_hosts[0].fingerprint, "SHA256:old");
    assert!(state.storage.known_hosts[0].trusted);
    assert!(state.backend_commands.is_empty());
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
