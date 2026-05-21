use super::*;

#[test]
fn backend_queue_pump_skips_terminal_sftp_list_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .sessions
        .set_sftp_loading_for_session(session_id, true);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::Sftp {
            session_id,
            request: crate::backend::SftpRequest::ListDir {
                remote_path: "/home/ops".to_owned(),
            },
        });
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(state.sessions.sftp_browsers[0].last_error.is_none());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn backend_queue_pump_marks_terminal_sftp_transfers_failed_without_executor() {
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
        .set_status(session_id, SessionStatus::Connected);
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::RemoteName,
        value: "app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let transfer_ids = state
        .sessions
        .transfers
        .iter()
        .map(|task| task.id)
        .collect::<Vec<_>>();
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert_eq!(
        state
            .sessions
            .transfers
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>(),
        transfer_ids
    );
    assert!(state.sessions.transfers.iter().all(|task| matches!(
        &task.status,
        TransferStatus::Failed { reason } if reason.contains("SFTP 会话已结束")
    )));
}

#[test]
fn backend_queue_pump_marks_terminal_sftp_write_commands_failed_without_executor() {
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
        .set_status(session_id, SessionStatus::Connected);
    state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: "/home/ops/old.log".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "releases".to_owned(),
    });
    state.apply(Message::CreateSftpDir { host_id });
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(
        state.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("SFTP 会话已结束")
    );
}

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

#[test]
fn backend_queue_pump_marks_sftp_transfer_failed_on_executor_error() {
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
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::RemoteName,
        value: "app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        &state.sessions.transfers[0].status,
        TransferStatus::Failed { reason } if reason.contains("不支持")
    ));
    assert_eq!(state.sessions.transfers[0].id, transfer_id);
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(
        state.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("不支持")
    );
}

#[test]
fn backend_queue_pump_keeps_sftp_session_connected_on_sftp_operation_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state
        .sessions
        .set_status(state.sessions.tabs[0].id, SessionStatus::Connected);
    state.apply(Message::RefreshSftp { host_id });
    let mut executor = FailingSftpExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.as_deref().unwrap_or("").contains("SFTP"));
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(
        state.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("permission denied")
    );
}

#[test]
fn backend_queue_pump_discards_pending_sftp_transfers_after_sftp_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state
        .sessions
        .set_status(state.sessions.tabs[0].id, SessionStatus::Connected);
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::RemoteName,
        value: "app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/metrics.log".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::RemoteName,
        value: "metrics.log".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    state.apply(Message::RefreshSftp { host_id });
    let mut executor = FailingSftpExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 3);
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(crate::backend::BackendCommand::Sftp {
            request: crate::backend::SftpRequest::ListDir { .. },
            ..
        })
    ));
    assert!(state.sessions.transfers.iter().all(|task| matches!(
        &task.status,
        TransferStatus::Failed { reason } if reason.contains("permission denied")
    )));
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
}

#[test]
fn backend_queue_pump_discards_pending_sftp_writes_after_sftp_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state
        .sessions
        .set_status(state.sessions.tabs[0].id, SessionStatus::Connected);
    state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: "/home/ops/old.log".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "releases".to_owned(),
    });
    state.apply(Message::CreateSftpDir { host_id });
    state.apply(Message::RefreshSftp { host_id });
    let mut executor = FailingSftpExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(crate::backend::BackendCommand::Sftp {
            request: crate::backend::SftpRequest::ListDir { .. },
            ..
        })
    ));
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
    assert!(
        state.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("permission denied")
    );
}

#[derive(Debug, Clone, Copy)]
struct FailingSftpExecutor;

impl BackendExecutor for FailingSftpExecutor {
    fn execute(
        &mut self,
        command: crate::backend::BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        assert!(matches!(
            command,
            crate::backend::BackendCommand::Sftp { .. }
        ));

        Err(BackendExecutionError::SftpFailed {
            operation: "list dir".to_owned(),
            reason: "permission denied".to_owned(),
        })
    }
}
