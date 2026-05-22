use super::*;

#[test]
fn backend_queue_pump_marks_pruned_sftp_transfer_failed_on_terminal_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.sessions.transfers[0].id, transfer_id);
    assert!(matches!(
        &state.sessions.transfers[0].status,
        TransferStatus::Failed { reason } if reason.contains("不支持")
    ));
    assert!(!state.sessions.sftp_browsers[0].loading);
}
