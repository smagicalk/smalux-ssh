use super::*;

#[test]
fn reconnect_shell_reuses_existing_disconnected_shell_tab() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let session_id = state.sessions.tabs[0].id;
    assert!(
        state
            .sessions
            .set_status(session_id, SessionStatus::Disconnected)
    );
    state.backend_commands.drain();

    let outcome = state.apply(Message::ReconnectShell { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.terminal.tab_count(), 1);
    assert_eq!(state.terminal.active_tab, Some(session_id));
    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Terminal);
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Reconnecting
    ));

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[0],
        BackendCommand::Connect {
            session_id: command_session_id,
            target,
        } if *command_session_id == session_id
            && target.host_id == host_id
            && target.endpoint() == "example.com:22"
    ));
    assert!(matches!(
        &commands[1],
        BackendCommand::OpenShell {
            session_id: command_session_id,
            pty,
        } if *command_session_id == session_id && pty.term == "xterm-256color"
    ));
}

#[test]
fn reconnect_shell_rejects_active_shell_without_queueing_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();

    let outcome = state.apply(Message::ReconnectShell { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connecting
    ));
}

#[test]
fn reconnect_shell_rejects_remote_command_tabs() {
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
    assert!(
        state
            .sessions
            .set_status(session_id, SessionStatus::Disconnected)
    );
    state.backend_commands.drain();

    let outcome = state.apply(Message::ReconnectShell { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}
