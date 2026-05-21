use super::*;

#[test]
fn cancel_sftp_transfer_cancels_queued_transfer_and_removes_backend_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    let transfer_id = state.sessions.transfers[0].id;

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert!(state.backend_commands.is_empty());
}

#[test]
fn cancel_sftp_transfer_keeps_same_id_command_from_other_session() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    let transfer_id = state.sessions.transfers[0].id;
    let owner_session_id = state.sessions.transfers[0].session_id;
    let stale_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.backend_commands.push(BackendCommand::Sftp {
        session_id: stale_session_id,
        request: crate::backend::SftpRequest::Download {
            id: transfer_id,
            remote_path: "/stale/deploy.sh".to_owned(),
            local_path: "C:/tmp/stale-deploy.sh".to_owned(),
        },
    });

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp {
            session_id,
            request: crate::backend::SftpRequest::Download { id, .. },
        }) if *session_id == stale_session_id && *id == transfer_id
    ));
    assert_ne!(owner_session_id, stale_session_id);
}

#[test]
fn cancel_sftp_transfer_rejects_ambiguous_same_id_tasks() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    let transfer_id = state.sessions.transfers[0].id;
    let current_session_id = state.sessions.transfers[0].session_id;
    let other_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let mut other_task = state.sessions.transfers[0].clone();
    other_task.session_id = other_session_id;
    other_task.local_path = "C:/tmp/other-deploy.sh".to_owned();
    state.sessions.transfers.push(other_task);

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("不唯一"));
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert_eq!(state.sessions.transfers[0].session_id, current_session_id);
    assert_eq!(state.sessions.transfers[1].session_id, other_session_id);
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Queued
    ));
    assert!(matches!(
        state.sessions.transfers[1].status,
        crate::model::TransferStatus::Queued
    ));
}

#[test]
fn cancel_sftp_transfer_rejects_transfer_already_removed_from_queue() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    let transfer_id = state.sessions.transfers[0].id;
    state.backend_commands.drain();

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("已经开始"));
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Queued
    ));
}
