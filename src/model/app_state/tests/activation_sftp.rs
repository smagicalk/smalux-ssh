use super::*;

#[test]
fn activate_sftp_tab_message_switches_session_without_terminal_tab() {
    let mut state = AppState::default();
    let first_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_session_id, host_id, "/");
    state
        .sessions
        .open_sftp_tab(second_session_id, host_id, "/var/log");

    let outcome = state.apply(Message::ActivateTerminalTab {
        session_id: first_session_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.active_tab, Some(first_session_id));
    assert!(state.terminal.active_tab.is_none());
}
