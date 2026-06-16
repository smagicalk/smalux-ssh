use super::*;

#[test]
fn backend_queue_pump_discards_pending_sftp_writes_after_sftp_error() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.core.backend_commands.drain();
    let session_id = state.core.sessions.tabs[0].id;
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state.apply_message(Message::RemoveSftpFile {
        host_id,
        remote_path: "/home/ops/old.log".to_owned(),
    });
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "releases".to_owned(),
    });
    state.apply_message(Message::CreateSftpDir { host_id });
    state.apply_message(Message::RefreshSftp { host_id });
    let mut executor = FailingSftpExecutor;

    let outcome = state.core.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert_eq!(state.core.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(crate::backend::BackendCommand::Sftp {
            request: crate::backend::SftpRequest::ListDir { .. },
            ..
        })
    ));
    assert!(matches!(
        state.core.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
    assert!(
        state.core.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("permission denied")
    );
}
