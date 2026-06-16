use super::*;

#[test]
fn save_sftp_bookmark_uses_current_browser_directory() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.core.backend_commands.drain();

    let outcome = state.apply_message(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert_eq!(state.core.storage.sftp_bookmark_count(), 1);
    assert_eq!(state.core.storage.sftp_bookmarks[0].host_id, host_id);
    assert_eq!(state.core.storage.sftp_bookmarks[0].label, "ops");
    assert_eq!(
        state.core.storage.sftp_bookmarks[0].remote_path,
        "/home/ops"
    );
}

#[test]
fn save_sftp_bookmark_rejects_disconnected_browser_without_bookmark() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.core.sessions.tabs[0].id;
    state.core.backend_commands.drain();
    state
        .core
        .sessions
        .set_status(session_id, crate::model::SessionStatus::Disconnected);

    let outcome = state.apply_message(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有可用的 SFTP 会话")
    );
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.core.storage.sftp_bookmark_count(), 0);
}

#[test]
fn save_sftp_bookmark_reassigns_disconnected_owner_before_saving() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let fallback_session_id = state.core.sessions.tabs[0].id;
    state.core.backend_commands.drain();
    state
        .core
        .sessions
        .set_status(fallback_session_id, crate::model::SessionStatus::Connected);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let disconnected_owner_id = state.core.sessions.tabs[1].id;
    state.core.backend_commands.drain();
    state.core.sessions.set_status(
        disconnected_owner_id,
        crate::model::SessionStatus::Disconnected,
    );

    let outcome = state.apply_message(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(
        state.core.sessions.sftp_browsers[0].session_id,
        fallback_session_id
    );
    assert_eq!(state.core.storage.sftp_bookmark_count(), 1);
    assert_eq!(state.core.storage.sftp_bookmarks[0].remote_path, "/var/log");
}

#[test]
fn save_sftp_bookmark_reports_missing_browser() {
    let mut state = desktop_state();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply_message(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.core.storage.sftp_bookmark_count(), 0);
}
