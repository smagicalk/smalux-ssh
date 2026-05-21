use super::*;

#[test]
fn send_terminal_input_message_queues_shell_input_and_records_history() {
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
    state.ui.set_terminal_input(session_id, "ls");

    let outcome = state.apply(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.storage.command_history_count(), 1);
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert!(matches!(
        state.backend_commands.front(),
        Some(crate::backend::BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "ls\n"
    ));
}

#[test]
fn send_remote_terminal_input_rejects_empty_command() {
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
    state.ui.set_terminal_input(session_id, "  ");

    let outcome = state.apply(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("不能为空"));
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.command_history_count(), 0);
}

#[test]
fn send_terminal_input_rejects_disconnected_or_failed_shell() {
    let mut state = AppState::default();
    let disconnected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let failed_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(disconnected_id, host_id, "disconnected");
    state.sessions.open_shell_tab(failed_id, host_id, "failed");
    assert!(
        state
            .sessions
            .set_status(disconnected_id, SessionStatus::Disconnected)
    );
    assert!(state.sessions.set_status(
        failed_id,
        SessionStatus::Failed {
            reason: "network".to_owned(),
        },
    ));
    state.ui.set_terminal_input(disconnected_id, "ls");
    state.ui.set_terminal_input(failed_id, "pwd");

    let disconnected = state.apply(Message::SendTerminalInput {
        session_id: disconnected_id,
    });
    let failed = state.apply(Message::SendTerminalInput {
        session_id: failed_id,
    });

    assert!(disconnected.changed());
    assert!(failed.changed());
    assert!(
        disconnected
            .error
            .as_deref()
            .unwrap_or("")
            .contains("不可交互")
    );
    assert!(failed.error.as_deref().unwrap_or("").contains("不可交互"));
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.command_history_count(), 0);
    assert_eq!(state.ui.terminal_input_for(disconnected_id), "ls");
    assert_eq!(state.ui.terminal_input_for(failed_id), "pwd");
}
