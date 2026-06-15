use super::*;

#[test]
fn backend_queue_pump_keeps_sftp_session_connected_on_sftp_operation_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    let session_id = state.sessions.tabs[0].id;
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state.apply(Message::RefreshSftp { host_id });
    let mut executor = FailingSftpExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.as_deref().unwrap_or("").contains("SFTP"));
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(
        state.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("permission denied")
    );
}
