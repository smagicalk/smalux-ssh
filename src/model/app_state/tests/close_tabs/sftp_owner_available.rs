use super::*;

#[test]
fn close_current_sftp_tab_reassigns_browser_to_available_session() {
    let mut state = core_state();
    let connected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let disconnected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(connected_id, host_id, "/home/ops");
    state
        .sessions
        .set_status(connected_id, SessionStatus::Connected);
    state
        .sessions
        .open_sftp_tab(disconnected_id, host_id, "/tmp");
    state
        .sessions
        .set_status(disconnected_id, SessionStatus::Disconnected);
    state
        .sessions
        .open_sftp_tab(current_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: current_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.sftp_browsers[0].session_id, connected_id);
    assert_eq!(state.sessions.tab_count(), 2);
}

#[test]
fn close_current_sftp_tab_removes_browser_when_only_disconnected_tabs_remain() {
    let mut state = core_state();
    let disconnected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(disconnected_id, host_id, "/tmp");
    state
        .sessions
        .set_status(disconnected_id, SessionStatus::Disconnected);
    state
        .sessions
        .open_sftp_tab(current_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: current_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 0);
}
