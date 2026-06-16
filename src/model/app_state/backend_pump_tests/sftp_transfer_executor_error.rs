use super::*;

#[test]
fn backend_queue_pump_marks_sftp_transfer_failed_on_executor_error() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.core.backend_commands.drain();
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
    let transfer_id = state.core.sessions.transfers[0].id;
    let mut executor = NoopBackendExecutor;

    let outcome = state.core.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(state.core.backend_commands.is_empty());
    assert!(matches!(
        &state.core.sessions.transfers[0].status,
        TransferStatus::Failed { reason } if reason.contains("不支持")
    ));
    assert_eq!(state.core.sessions.transfers[0].id, transfer_id);
    assert!(!state.core.sessions.sftp_browsers[0].loading);
    assert!(
        state.core.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("不支持")
    );
}
