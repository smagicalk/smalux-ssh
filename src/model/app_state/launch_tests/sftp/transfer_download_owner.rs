use super::*;

#[test]
fn download_sftp_message_keeps_browser_loading_unchanged() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    assert!(
        state
            .sessions
            .set_sftp_entries(host_id, "/home/ops", Vec::new())
    );
    assert!(!state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(!state.sessions.sftp_browsers[0].loading);
}

#[test]
fn download_sftp_invalid_remote_path_does_not_reassign_browser_owner() {
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

    let outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: " / ".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("根目录"));
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        disconnected_owner_id
    );
    assert_eq!(state.sessions.transfer_count(), 0);
    assert!(state.backend_commands.is_empty());
}
