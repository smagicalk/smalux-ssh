use super::*;

#[test]
fn close_session_tab_message_removes_last_sftp_browser_for_host() {
    let mut state = core_state();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.sessions.sftp_browser_count(), 0);
}

#[test]
fn close_session_tab_message_keeps_sftp_browser_when_same_host_tab_remains() {
    let mut state = core_state();
    let first_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_id, host_id, "/home/ops");
    state.sessions.open_sftp_tab(second_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: first_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/var/log");
}
