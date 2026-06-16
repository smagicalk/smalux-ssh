use super::*;

#[test]
fn backend_queue_pump_marks_terminal_sftp_transfers_failed_without_executor() {
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
        .set_status(session_id, SessionStatus::Connected);
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::RemoteName,
        value: "app.tar.gz".to_owned(),
    });
    state.apply_message(Message::UploadSftp { host_id });
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply_message(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let transfer_ids = state
        .core
        .sessions
        .transfers
        .iter()
        .map(|task| task.id)
        .collect::<Vec<_>>();
    let mut executor = NoopBackendExecutor;

    let outcome = state.core.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(outcome.error.is_none());
    assert!(state.core.backend_commands.is_empty());
    assert!(!state.core.sessions.sftp_browsers[0].loading);
    assert_eq!(
        state
            .core
            .sessions
            .transfers
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>(),
        transfer_ids
    );
    assert!(state.core.sessions.transfers.iter().all(|task| matches!(
        &task.status,
        TransferStatus::Failed { reason } if reason.contains("SFTP 会话已结束")
    )));
}
