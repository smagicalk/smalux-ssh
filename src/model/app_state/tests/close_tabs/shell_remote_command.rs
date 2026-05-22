use super::*;

#[test]
fn close_pending_remote_command_tab_finishes_history_without_exit_code() {
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

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.command_history[0].exit_code, None);
    assert!(state.storage.command_history[0].duration_ms.is_some());
}
