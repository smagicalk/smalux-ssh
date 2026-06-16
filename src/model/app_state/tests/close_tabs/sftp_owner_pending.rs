use super::*;

#[test]
fn close_current_pending_sftp_tab_reassigns_browser_without_loading() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let first_id = state.core.sessions.tabs[0].id;
    state.core.backend_commands.drain();
    state
        .core
        .sessions
        .set_status(first_id, SessionStatus::Connected);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let second_id = state.core.sessions.tabs[1].id;
    assert!(state.core.sessions.sftp_browsers[0].loading);

    let outcome = state.apply_message(Message::CloseSessionTab {
        session_id: second_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.core.sessions.tab_count(), 1);
    assert_eq!(state.core.sessions.sftp_browser_count(), 1);
    assert_eq!(state.core.sessions.sftp_browsers[0].session_id, first_id);
    assert!(!state.core.sessions.sftp_browsers[0].loading);
    assert!(state.core.backend_commands.is_empty());
}
