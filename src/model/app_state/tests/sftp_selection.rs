use super::*;

#[test]
fn select_sftp_entry_message_updates_browser_selection() {
    let mut state = core_state();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");

    let outcome = state.apply(Message::SelectSftpEntry {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(
        state.sessions.sftp_browsers[0].selected_path.as_deref(),
        Some("/home/ops/deploy.sh")
    );
    assert!(state.backend_commands.is_empty());
}

#[test]
fn select_sftp_entry_reassigns_disconnected_browser_owner() {
    let mut state = core_state();
    let host = sample_host();
    let host_id = host.id;
    let fallback_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let disconnected_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_sftp_tab(fallback_session_id, host_id, "/home/ops");
    state
        .sessions
        .set_status(fallback_session_id, SessionStatus::Connected);
    state
        .sessions
        .open_sftp_tab(disconnected_session_id, host_id, "/var/log");
    state
        .sessions
        .set_status(disconnected_session_id, SessionStatus::Disconnected);

    let outcome = state.apply(Message::SelectSftpEntry {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        fallback_session_id
    );
    assert_eq!(
        state.sessions.sftp_browsers[0].selected_path.as_deref(),
        Some("/home/ops/deploy.sh")
    );
    assert!(state.backend_commands.is_empty());
}

#[test]
fn select_sftp_entry_rejects_disconnected_browser_without_fallback_session() {
    let mut state = core_state();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);

    let outcome = state.apply(Message::SelectSftpEntry {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有可用的 SFTP 会话")
    );
    assert!(state.sessions.sftp_browsers[0].selected_path.is_none());
    assert!(state.backend_commands.is_empty());
}
