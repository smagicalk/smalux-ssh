use super::*;

#[test]
fn activate_terminal_tab_message_switches_active_tab() {
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

    let outcome = state.apply(Message::ActivateTerminalTab { session_id });

    assert!(outcome.changed());
    assert_eq!(state.terminal.active_tab, Some(session_id));
    assert_eq!(state.sessions.active_tab, Some(session_id));
}
