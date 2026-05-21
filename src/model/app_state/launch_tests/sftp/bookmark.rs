use super::*;

#[test]
fn save_sftp_bookmark_uses_current_browser_directory() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let outcome = state.apply(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert_eq!(state.storage.sftp_bookmark_count(), 1);
    assert_eq!(state.storage.sftp_bookmarks[0].host_id, host_id);
    assert_eq!(state.storage.sftp_bookmarks[0].label, "ops");
    assert_eq!(state.storage.sftp_bookmarks[0].remote_path, "/home/ops");
}

#[test]
fn save_sftp_bookmark_rejects_disconnected_browser_without_bookmark() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(session_id, crate::model::SessionStatus::Disconnected);

    let outcome = state.apply(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有可用的 SFTP 会话")
    );
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.storage.sftp_bookmark_count(), 0);
}

#[test]
fn save_sftp_bookmark_reassigns_disconnected_owner_before_saving() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let fallback_session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(fallback_session_id, crate::model::SessionStatus::Connected);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let disconnected_owner_id = state.sessions.tabs[1].id;
    state.backend_commands.drain();
    state.sessions.set_status(
        disconnected_owner_id,
        crate::model::SessionStatus::Disconnected,
    );

    let outcome = state.apply(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        fallback_session_id
    );
    assert_eq!(state.storage.sftp_bookmark_count(), 1);
    assert_eq!(state.storage.sftp_bookmarks[0].remote_path, "/var/log");
}

#[test]
fn save_sftp_bookmark_reports_missing_browser() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.storage.sftp_bookmark_count(), 0);
}

#[test]
fn open_sftp_bookmark_opens_browser_when_none_exists() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::OpenSftpBookmark {
        host_id,
        remote_path: "/var/log".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/var/log");
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Connect { target, .. }) if target.host_id == host_id
    ));
}

#[test]
fn open_sftp_bookmark_navigates_existing_browser() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let outcome = state.apply(Message::OpenSftpBookmark {
        host_id,
        remote_path: "/etc".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(state.sessions.sftp_browsers[0].loading);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. }) if request.remote_path() == "/etc"
    ));
}

#[test]
fn open_sftp_bookmark_reopens_disconnected_browser() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let old_session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(old_session_id, crate::model::SessionStatus::Disconnected);

    let outcome = state.apply(Message::OpenSftpBookmark {
        host_id,
        remote_path: "/etc".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 2);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/etc");
    let new_session_id = state.sessions.tabs[1].id;
    assert_ne!(old_session_id, new_session_id);
    assert!(matches!(
        state.backend_commands.drain().as_slice(),
        [
            BackendCommand::Connect {
                session_id: connect_session_id,
                ..
            },
            BackendCommand::Sftp {
                session_id: sftp_session_id,
                request,
            },
        ] if *connect_session_id == new_session_id
            && *sftp_session_id == new_session_id
            && request.remote_path() == "/etc"
    ));
}

#[test]
fn remove_sftp_bookmark_updates_storage_or_reports_missing() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());
    state
        .storage
        .upsert_sftp_bookmark(crate::model::SftpBookmark {
            host_id,
            label: "logs".to_owned(),
            remote_path: "/var/log".to_owned(),
        });

    let remove_outcome = state.apply(Message::RemoveSftpBookmark {
        host_id,
        remote_path: "/var/log".to_owned(),
    });
    let missing_outcome = state.apply(Message::RemoveSftpBookmark {
        host_id,
        remote_path: "/var/log".to_owned(),
    });

    assert!(remove_outcome.changed());
    assert_eq!(state.storage.sftp_bookmark_count(), 0);
    assert!(missing_outcome.changed());
    assert!(missing_outcome.error.is_some());
}
