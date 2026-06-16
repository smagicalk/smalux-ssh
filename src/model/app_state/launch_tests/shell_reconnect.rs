use super::*;

#[test]
fn reconnect_shell_rejects_active_shell_without_queueing_commands() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::OpenShell { host_id });
    let session_id = state.core.sessions.tabs[0].id;
    state.core.backend_commands.drain();

    let outcome = state.apply_message(Message::ReconnectShell { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert!(state.core.backend_commands.is_empty());
    assert!(matches!(
        state.core.sessions.tabs[0].status,
        SessionStatus::Connecting
    ));
}

#[test]
fn reconnect_shell_rejects_remote_command_tabs() {
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
    assert!(
        state
            .core
            .sessions
            .set_status(session_id, SessionStatus::Disconnected)
    );
    state.core.backend_commands.drain();

    let outcome = state.apply_message(Message::ReconnectShell { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert!(state.core.backend_commands.is_empty());
    assert!(matches!(
        state.core.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}
