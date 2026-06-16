use super::*;

#[test]
fn cancel_stale_sftp_upload_keeps_current_browser_loading() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.core.backend_commands.drain();
    assert!(
        state
            .core
            .sessions
            .set_sftp_entries(host_id, "/home/ops", Vec::new())
    );
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply_message(Message::UploadSftp { host_id });
    let transfer_id = state.core.sessions.transfers[0].id;
    let stale_session_id = state.core.sessions.transfers[0].session_id;
    state.core.backend_commands.drain();
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let current_session_id = state.core.sessions.sftp_browsers[0].session_id;
    state.core.backend_commands.push(BackendCommand::Sftp {
        session_id: stale_session_id,
        request: crate::backend::SftpRequest::Upload {
            id: transfer_id,
            local_path: "C:/tmp/app.tar.gz".to_owned(),
            remote_path: "/home/ops/app.tar.gz".to_owned(),
        },
    });
    assert_ne!(stale_session_id, current_session_id);
    assert!(state.core.sessions.sftp_browsers[0].loading);

    let outcome = state.apply_message(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(state.core.sessions.sftp_browsers[0].loading);
    assert_eq!(
        state.core.sessions.sftp_browsers[0].session_id,
        current_session_id
    );
    assert!(matches!(
        state.core.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
}

#[test]
fn cancel_sftp_upload_keeps_browser_loading_when_another_refresh_request_remains() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.core.backend_commands.drain();
    assert!(
        state
            .core
            .sessions
            .set_sftp_entries(host_id, "/home/ops", Vec::new())
    );
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply_message(Message::UploadSftp { host_id });
    let first_transfer_id = state.core.sessions.transfers[0].id;
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/assets.tar.gz".to_owned(),
    });
    state.apply_message(Message::UploadSftp { host_id });
    assert_eq!(state.core.backend_commands.pending_count(), 2);

    let outcome = state.apply_message(Message::CancelSftpTransfer {
        transfer_id: first_transfer_id,
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(state.core.backend_commands.pending_count(), 1);
    assert!(state.core.sessions.sftp_browsers[0].loading);
    assert!(matches!(
        state.core.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert!(matches!(
        state.core.sessions.transfers[1].status,
        crate::model::TransferStatus::Queued
    ));
}
