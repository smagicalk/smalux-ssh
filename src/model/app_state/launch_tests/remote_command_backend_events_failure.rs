use super::*;

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
