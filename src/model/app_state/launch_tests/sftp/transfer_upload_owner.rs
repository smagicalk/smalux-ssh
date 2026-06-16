use super::*;

#[test]
fn upload_sftp_reassigns_browser_owner_before_setting_loading() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let fallback_session_id = state.core.sessions.tabs[0].id;
    state.core.backend_commands.drain();
    state
        .core
        .sessions
        .set_status(fallback_session_id, crate::model::SessionStatus::Connected);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let disconnected_owner_id = state.core.sessions.tabs[1].id;
    state.core.backend_commands.drain();
    state.core.sessions.set_status(
        disconnected_owner_id,
        crate::model::SessionStatus::Disconnected,
    );
    assert!(
        state
            .core
            .sessions
            .set_sftp_entries(host_id, "/home/ops", Vec::new())
    );
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });

    let outcome = state.apply_message(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(
        state.core.sessions.sftp_browsers[0].session_id,
        fallback_session_id
    );
    assert!(state.core.sessions.sftp_browsers[0].loading);
    assert_eq!(
        state.core.sessions.transfers[0].session_id,
        fallback_session_id
    );
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::Sftp { session_id, .. })
            if *session_id == fallback_session_id
    ));
}

#[test]
fn upload_sftp_invalid_input_does_not_reassign_browser_owner() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let fallback_session_id = state.core.sessions.tabs[0].id;
    state.core.backend_commands.drain();
    state
        .core
        .sessions
        .set_status(fallback_session_id, crate::model::SessionStatus::Connected);
    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let disconnected_owner_id = state.core.sessions.tabs[1].id;
    state.core.backend_commands.drain();
    state.core.sessions.set_status(
        disconnected_owner_id,
        crate::model::SessionStatus::Disconnected,
    );

    let outcome = state.apply_message(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("本地路径不能为空")
    );
    assert_eq!(
        state.core.sessions.sftp_browsers[0].session_id,
        disconnected_owner_id
    );
    assert_eq!(state.core.sessions.transfer_count(), 0);
    assert!(state.core.backend_commands.is_empty());
}
