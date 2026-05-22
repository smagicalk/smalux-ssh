use super::*;

#[test]
fn close_pending_shell_tab_removes_launch_commands_without_disconnect() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let session_id = state.sessions.tabs[0].id;
    assert_eq!(state.backend_commands.pending_count(), 2);

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
}
