use super::*;

#[test]
fn backend_queue_pump_skips_terminal_sftp_list_commands() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.core.storage.upsert_host(host);
    state
        .core
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .core
        .sessions
        .set_sftp_loading_for_session(session_id, true);
    state
        .core
        .backend_commands
        .push(crate::backend::BackendCommand::Sftp {
            session_id,
            request: crate::backend::SftpRequest::ListDir {
                remote_path: "/home/ops".to_owned(),
            },
        });
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let mut executor = NoopBackendExecutor;

    let outcome = state.core.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.core.backend_commands.is_empty());
    assert!(!state.core.sessions.sftp_browsers[0].loading);
    assert!(state.core.sessions.sftp_browsers[0].last_error.is_none());
    assert!(matches!(
        state.core.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn backend_queue_pump_marks_terminal_sftp_write_commands_failed_without_executor() {
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
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let mut executor = NoopBackendExecutor;

    let outcome = state.core.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.core.backend_commands.is_empty());
    assert!(!state.core.sessions.sftp_browsers[0].loading);
    assert!(
        state.core.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("SFTP 会话已结束")
    );
}
