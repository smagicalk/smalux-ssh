use super::*;

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
