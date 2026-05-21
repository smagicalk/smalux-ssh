use super::*;

#[test]
fn cancel_sftp_upload_clears_browser_loading_when_queued_request_is_removed() {
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
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    assert!(state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
}

#[test]
fn cancel_sftp_upload_ignores_stale_session_refresh_when_clearing_loading() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let stale_session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let current_session_id = state.sessions.tabs[1].id;
    state.backend_commands.drain();
    assert_ne!(stale_session_id, current_session_id);
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        current_session_id
    );
    assert!(state.sessions.set_sftp_entries_for_session(
        current_session_id,
        "/var/log",
        Vec::new()
    ));
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    state.backend_commands.push(BackendCommand::Sftp {
        session_id: stale_session_id,
        request: crate::backend::SftpRequest::ListDir {
            remote_path: "/home/ops".to_owned(),
        },
    });
    assert!(state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        current_session_id
    );
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp {
            session_id,
            request: crate::backend::SftpRequest::ListDir { .. },
        }) if *session_id == stale_session_id
    ));
}
