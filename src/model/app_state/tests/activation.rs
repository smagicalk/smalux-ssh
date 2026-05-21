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

#[test]
fn activate_sftp_tab_message_reassigns_browser_owner() {
    let mut state = AppState::default();
    let first_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_session_id, host_id, "/");
    state
        .sessions
        .open_sftp_tab(second_session_id, host_id, "/var/log");
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        second_session_id
    );

    let outcome = state.apply(Message::ActivateTerminalTab {
        session_id: first_session_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.active_tab, Some(first_session_id));
    assert_eq!(state.sessions.sftp_browsers[0].session_id, first_session_id);
}

#[test]
fn activate_disconnected_sftp_tab_keeps_available_browser_owner() {
    let mut state = AppState::default();
    let connected_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let disconnected_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(disconnected_session_id, host_id, "/old");
    state
        .sessions
        .set_status(disconnected_session_id, SessionStatus::Disconnected);
    state
        .sessions
        .open_sftp_tab(connected_session_id, host_id, "/current");
    state
        .sessions
        .set_status(connected_session_id, SessionStatus::Connected);

    let outcome = state.apply(Message::ActivateTerminalTab {
        session_id: disconnected_session_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.active_tab, Some(disconnected_session_id));
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        connected_session_id
    );
}
