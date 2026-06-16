use super::*;

#[test]
fn close_pending_remote_command_tab_finishes_history_without_exit_code() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::RunRemoteCommand {
        host_id,
        command: "uptime".to_owned(),
        request_pty: false,
    });
    let session_id = state.core.sessions.tabs[0].id;
    state.core.storage.command_history[0].started_at_unix_secs = 1;

    let outcome = state.apply_message(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.core.sessions.tab_count(), 0);
    assert_eq!(state.core.terminal.tab_count(), 0);
    assert!(state.core.backend_commands.is_empty());
    assert_eq!(state.core.storage.command_history[0].exit_code, None);
    assert!(state.core.storage.command_history[0].duration_ms.is_some());
}
