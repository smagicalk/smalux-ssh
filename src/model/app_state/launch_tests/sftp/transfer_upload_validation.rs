use super::*;

#[test]
fn upload_sftp_rejects_disconnected_session_without_transfer() {
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
        .set_status(session_id, crate::model::SessionStatus::Disconnected);
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });

    let outcome = state.apply_message(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有可用的 SFTP 会话")
    );
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.core.sessions.transfer_count(), 0);
    assert!(state.core.backend_commands.is_empty());
}

#[test]
fn upload_sftp_rejects_path_like_remote_name() {
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
        value: "../app.tar.gz".to_owned(),
    });

    let outcome = state.apply_message(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("路径分隔符")
    );
    assert_eq!(state.core.sessions.transfer_count(), 0);
    assert!(state.core.backend_commands.is_empty());
}
