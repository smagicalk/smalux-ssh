use super::*;

#[test]
fn close_pending_sftp_tab_cancels_queued_transfer_and_removes_commands() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.core.sessions.tabs[0].id;
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply_message(Message::UploadSftp { host_id });
    assert_eq!(state.core.backend_commands.pending_count(), 3);
    assert!(matches!(
        state.core.sessions.transfers[0].status,
        crate::model::TransferStatus::Queued
    ));

    let outcome = state.apply_message(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.core.sessions.tab_count(), 0);
    assert_eq!(state.core.sessions.sftp_browser_count(), 0);
    assert!(state.core.backend_commands.is_empty());
    assert!(matches!(
        state.core.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
}

#[test]
fn close_pending_sftp_tab_keeps_same_id_transfer_from_other_session() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.core.sessions.tabs[0].id;
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply_message(Message::UploadSftp { host_id });
    let transfer_id = state.core.sessions.transfers[0].id;
    let stale_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let mut stale_transfer = state.core.sessions.transfers[0].clone();
    stale_transfer.session_id = stale_session_id;
    stale_transfer.local_path = "C:/tmp/stale-app.tar.gz".to_owned();
    state.core.sessions.transfers.push(stale_transfer);

    let outcome = state.apply_message(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(state.core.backend_commands.is_empty());
    assert!(matches!(
        state.core.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert_eq!(state.core.sessions.transfers[1].id, transfer_id);
    assert_eq!(
        state.core.sessions.transfers[1].session_id,
        stale_session_id
    );
    assert!(matches!(
        state.core.sessions.transfers[1].status,
        crate::model::TransferStatus::Queued
    ));
}
