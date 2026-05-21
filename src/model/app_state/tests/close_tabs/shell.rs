use super::*;

#[test]
fn close_session_tab_message_closes_shell_and_queues_disconnect() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .terminal
        .open_tab(crate::terminal::TerminalTabState::new(
            session_id,
            "production",
        ));

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Disconnect { session_id: queued_session_id })
            if *queued_session_id == session_id
    ));
}

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

#[test]
fn close_session_tab_message_reports_missing_tab() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.state_changed);
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert!(state.backend_commands.is_empty());

    let dismiss_outcome = state.apply(Message::DismissUiError);

    assert!(dismiss_outcome.changed());
    assert!(state.ui.last_error.is_none());
}
