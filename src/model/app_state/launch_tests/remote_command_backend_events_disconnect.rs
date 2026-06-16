use super::*;

#[test]
fn remote_command_disconnect_marks_history_finished_without_exit_code() {
    let mut state = CoreState::default();
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
