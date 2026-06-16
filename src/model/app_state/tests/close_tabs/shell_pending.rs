use super::*;

#[test]
fn close_pending_shell_tab_removes_launch_commands_without_disconnect() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::OpenShell { host_id });
    let session_id = state.core.sessions.tabs[0].id;
    assert_eq!(state.core.backend_commands.pending_count(), 2);

    let outcome = state.apply_message(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.core.sessions.tab_count(), 0);
    assert_eq!(state.core.terminal.tab_count(), 0);
    assert!(state.core.backend_commands.is_empty());
}

#[test]
fn close_pending_local_shell_tab_removes_launch_command_without_disconnect() {
    let mut state = desktop_state();
    state.apply_message(Message::OpenLocalTerminal);
    let session_id = state.core.sessions.active_tab.expect("本地终端应已打开");
    assert_eq!(state.core.backend_commands.pending_count(), 1);

    let outcome = state.apply_message(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.core.sessions.tab_count(), 0);
    assert_eq!(state.core.terminal.tab_count(), 0);
    assert!(state.core.backend_commands.is_empty());
}
