use super::*;

#[test]
fn close_session_tab_message_removes_last_sftp_browser_for_host() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.sessions.sftp_browser_count(), 0);
}

#[test]
fn close_pending_sftp_tab_cancels_queued_transfer_and_removes_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    assert_eq!(state.backend_commands.pending_count(), 3);
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Queued
    ));

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.sessions.sftp_browser_count(), 0);
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
}

#[test]
fn close_pending_sftp_tab_keeps_same_id_transfer_from_other_session() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    let stale_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let mut stale_transfer = state.sessions.transfers[0].clone();
    stale_transfer.session_id = stale_session_id;
    stale_transfer.local_path = "C:/tmp/stale-app.tar.gz".to_owned();
    state.sessions.transfers.push(stale_transfer);

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert_eq!(state.sessions.transfers[1].id, transfer_id);
    assert_eq!(state.sessions.transfers[1].session_id, stale_session_id);
    assert!(matches!(
        state.sessions.transfers[1].status,
        crate::model::TransferStatus::Queued
    ));
}

#[test]
fn close_session_tab_message_keeps_sftp_browser_when_same_host_tab_remains() {
    let mut state = AppState::default();
    let first_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_id, host_id, "/home/ops");
    state.sessions.open_sftp_tab(second_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: first_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/var/log");
}

#[test]
fn close_stale_sftp_tab_keeps_current_browser_owner() {
    let mut state = AppState::default();
    let first_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_id, host_id, "/home/ops");
    state.sessions.open_sftp_tab(second_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: first_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].session_id, second_id);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/var/log");
}

#[test]
fn close_current_sftp_tab_reassigns_browser_owner() {
    let mut state = AppState::default();
    let first_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_id, host_id, "/home/ops");
    state.sessions.open_sftp_tab(second_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: second_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].session_id, first_id);
    assert!(
        state
            .sessions
            .set_sftp_entries_for_session(first_id, "/home/ops", Vec::new())
    );
}

#[test]
fn close_current_pending_sftp_tab_reassigns_browser_without_loading() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let first_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(first_id, SessionStatus::Connected);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let second_id = state.sessions.tabs[1].id;
    assert!(state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: second_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].session_id, first_id);
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn close_current_sftp_tab_reassigns_browser_to_available_session() {
    let mut state = AppState::default();
    let connected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let disconnected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(connected_id, host_id, "/home/ops");
    state
        .sessions
        .set_status(connected_id, SessionStatus::Connected);
    state
        .sessions
        .open_sftp_tab(disconnected_id, host_id, "/tmp");
    state
        .sessions
        .set_status(disconnected_id, SessionStatus::Disconnected);
    state
        .sessions
        .open_sftp_tab(current_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: current_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.sftp_browsers[0].session_id, connected_id);
    assert_eq!(state.sessions.tab_count(), 2);
}

#[test]
fn close_current_sftp_tab_removes_browser_when_only_disconnected_tabs_remain() {
    let mut state = AppState::default();
    let disconnected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(disconnected_id, host_id, "/tmp");
    state
        .sessions
        .set_status(disconnected_id, SessionStatus::Disconnected);
    state
        .sessions
        .open_sftp_tab(current_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: current_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 0);
}
