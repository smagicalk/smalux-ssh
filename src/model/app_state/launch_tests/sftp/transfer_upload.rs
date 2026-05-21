use super::*;

#[test]
fn upload_sftp_message_queues_transfer_and_upload_request() {
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
        value: "release.tar.gz".to_owned(),
    });

    let outcome = state.apply(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.transfer_count(), 1);
    assert!(matches!(
        &state.sessions.transfers[0].direction,
        crate::model::TransferDirection::Upload
    ));
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/release.tar.gz"
    ));
}

#[test]
fn upload_sftp_reassigns_browser_owner_before_setting_loading() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let fallback_session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(fallback_session_id, crate::model::SessionStatus::Connected);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let disconnected_owner_id = state.sessions.tabs[1].id;
    state.backend_commands.drain();
    state.sessions.set_status(
        disconnected_owner_id,
        crate::model::SessionStatus::Disconnected,
    );
    assert!(
        state
            .sessions
            .set_sftp_entries(host_id, "/home/ops", Vec::new())
    );
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });

    let outcome = state.apply(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        fallback_session_id
    );
    assert!(state.sessions.sftp_browsers[0].loading);
    assert_eq!(state.sessions.transfers[0].session_id, fallback_session_id);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { session_id, .. })
            if *session_id == fallback_session_id
    ));
}

#[test]
fn upload_sftp_invalid_input_does_not_reassign_browser_owner() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let fallback_session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(fallback_session_id, crate::model::SessionStatus::Connected);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let disconnected_owner_id = state.sessions.tabs[1].id;
    state.backend_commands.drain();
    state.sessions.set_status(
        disconnected_owner_id,
        crate::model::SessionStatus::Disconnected,
    );

    let outcome = state.apply(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("本地路径不能为空")
    );
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        disconnected_owner_id
    );
    assert_eq!(state.sessions.transfer_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn upload_sftp_rejects_disconnected_session_without_transfer() {
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
        .set_status(session_id, crate::model::SessionStatus::Disconnected);
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });

    let outcome = state.apply(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有可用的 SFTP 会话")
    );
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.transfer_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn upload_sftp_rejects_path_like_remote_name() {
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
        value: "../app.tar.gz".to_owned(),
    });

    let outcome = state.apply(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("路径分隔符")
    );
    assert_eq!(state.sessions.transfer_count(), 0);
    assert!(state.backend_commands.is_empty());
}
