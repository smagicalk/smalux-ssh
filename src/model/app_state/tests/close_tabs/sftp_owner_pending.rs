use super::*;

#[test]
fn close_current_pending_sftp_tab_reassigns_browser_without_loading() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let first_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(first_id, SessionStatus::Connected);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let second_id = state.sessions.tabs[1].id;
    assert!(state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: second_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].session_id, first_id);
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(state.backend_commands.is_empty());
}
