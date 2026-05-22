use super::*;

#[test]
fn close_current_sftp_tab_reassigns_browser_owner() {
    let mut state = AppState::default();
    let first_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_id, host_id, "/home/ops");
    state.sessions.open_sftp_tab(second_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: second_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].session_id, first_id);
    assert!(
        state
            .sessions
            .set_sftp_entries_for_session(first_id, "/home/ops", Vec::new())
    );
}
