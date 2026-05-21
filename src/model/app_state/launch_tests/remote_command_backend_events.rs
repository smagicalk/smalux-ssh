use super::*;

#[test]
fn remote_command_exit_updates_latest_history_exit_code() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RunRemoteCommand {
        host_id,
        command: "systemctl is-active sshd".to_owned(),
        request_pty: false,
    });
    let session_id = state.sessions.tabs[0].id;
    assert_eq!(state.storage.command_history[0].exit_code, None);
    state.storage.command_history[0].started_at_unix_secs = 1;

    let outcome = state.apply(Message::BackendEventReceived(
        crate::backend::BackendEvent::CommandExited {
            session_id,
            exit_code: Some(3),
        },
    ));

    assert!(outcome.changed());
    assert_eq!(state.storage.command_history[0].exit_code, Some(3));
    assert!(matches!(
        state.storage.command_history[0].duration_ms,
        Some(duration_ms) if duration_ms > 0
    ));
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Failed { .. }
    ));
}

#[test]
fn remote_command_exit_updates_history_by_session_history_id() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    for _ in 0..2 {
        state.apply(Message::RunRemoteCommand {
            host_id,
            command: "uptime".to_owned(),
            request_pty: false,
        });
    }
    let first_session_id = state.sessions.tabs[0].id;
    assert_eq!(state.storage.command_history_count(), 2);
    assert_eq!(state.storage.command_history[0].exit_code, None);
    assert_eq!(state.storage.command_history[1].exit_code, None);
    state.storage.command_history[0].started_at_unix_secs = 1;
    state.storage.command_history[1].started_at_unix_secs = 1;

    let outcome = state.apply(Message::BackendEventReceived(
        crate::backend::BackendEvent::CommandExited {
            session_id: first_session_id,
            exit_code: Some(7),
        },
    ));

    assert!(outcome.changed());
    assert_eq!(state.storage.command_history[0].exit_code, Some(7));
    assert_eq!(state.storage.command_history[1].exit_code, None);
    assert!(state.storage.command_history[0].duration_ms.is_some());
    assert_eq!(state.storage.command_history[1].duration_ms, None);
}

#[test]
fn remote_command_exit_keeps_legacy_history_fallback() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state.record_command_history(host_id, "uptime".to_owned());
    state.storage.command_history[0].started_at_unix_secs = 1;
    state
        .sessions
        .open_remote_command_tab(session_id, host_id, "uptime", None);

    let outcome = state.apply(Message::BackendEventReceived(
        crate::backend::BackendEvent::CommandExited {
            session_id,
            exit_code: Some(0),
        },
    ));

    assert!(outcome.changed());
    assert_eq!(state.storage.command_history[0].exit_code, Some(0));
    assert!(state.storage.command_history[0].duration_ms.is_some());
}

#[test]
fn remote_command_failure_marks_history_finished_without_exit_code() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RunRemoteCommand {
        host_id,
        command: "uptime".to_owned(),
        request_pty: false,
    });
    let session_id = state.sessions.tabs[0].id;
    state.storage.command_history[0].started_at_unix_secs = 1;

    let outcome = state.apply(Message::BackendEventReceived(
        crate::backend::BackendEvent::Failed {
            session_id,
            reason: "connection reset".to_owned(),
        },
    ));

    assert!(outcome.changed());
    assert_eq!(state.storage.command_history[0].exit_code, None);
    assert!(state.storage.command_history[0].duration_ms.is_some());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Failed { .. }
    ));
}

#[test]
fn remote_command_history_ignores_late_exit_after_failure() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RunRemoteCommand {
        host_id,
        command: "uptime".to_owned(),
        request_pty: false,
    });
    let session_id = state.sessions.tabs[0].id;
    state.storage.command_history[0].started_at_unix_secs = 1;

    state.apply(Message::BackendEventReceived(
        crate::backend::BackendEvent::Failed {
            session_id,
            reason: "connection reset".to_owned(),
        },
    ));
    let finished_duration = state.storage.command_history[0].duration_ms;
    let late_outcome = state.apply(Message::BackendEventReceived(
        crate::backend::BackendEvent::CommandExited {
            session_id,
            exit_code: Some(0),
        },
    ));

    assert!(!late_outcome.changed());
    assert_eq!(state.storage.command_history[0].exit_code, None);
    assert_eq!(
        state.storage.command_history[0].duration_ms,
        finished_duration
    );
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Failed { .. }
    ));
}

#[test]
fn remote_command_disconnect_marks_history_finished_without_exit_code() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RunRemoteCommand {
        host_id,
        command: "uptime".to_owned(),
        request_pty: false,
    });
    let session_id = state.sessions.tabs[0].id;
    state.storage.command_history[0].started_at_unix_secs = 1;

    let outcome = state.apply(Message::BackendEventReceived(
        crate::backend::BackendEvent::Disconnected { session_id },
    ));

    assert!(outcome.changed());
    assert_eq!(state.storage.command_history[0].exit_code, None);
    assert!(state.storage.command_history[0].duration_ms.is_some());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}
