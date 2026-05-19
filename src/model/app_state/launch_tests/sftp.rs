use super::*;

#[test]
fn open_sftp_message_creates_browser_and_queues_list_dir() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::OpenSftp {
        host_id,
        initial_dir: " /var/log ".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Sftp);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/var/log");
    assert!(state.sessions.sftp_browsers[0].loading);
    assert_eq!(state.storage.recent_count(), 1);

    let commands = state.backend_commands.drain();
    let session_id = state.sessions.tabs[0].id;
    assert!(matches!(
        &commands[0],
        BackendCommand::Connect {
            session_id: command_session_id,
            target,
        } if *command_session_id == session_id
            && target.host_id == host_id
    ));
    assert!(matches!(
        &commands[1],
        BackendCommand::Sftp {
            session_id: command_session_id,
            request,
        } if *command_session_id == session_id
            && request.remote_path() == "/var/log"
    ));
}

#[test]
fn open_sftp_defaults_empty_initial_dir_to_root() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: " ".to_owned(),
    });

    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/");
    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[1],
        BackendCommand::Sftp { request, .. } if request.remote_path() == "/"
    ));
}

#[test]
fn open_sftp_reports_missing_host_without_queueing_commands() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.sessions.sftp_browser_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn refresh_sftp_message_queues_current_directory_listing() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    state.backend_commands.drain();

    let outcome = state.apply(Message::RefreshSftp { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(state.sessions.sftp_browsers[0].loading);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/var/log"
    ));
}

#[test]
fn refresh_sftp_rejects_disconnected_browser_without_queueing_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(session_id, crate::model::SessionStatus::Disconnected);

    let outcome = state.apply(Message::RefreshSftp { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有可用的 SFTP 会话")
    );
    assert_eq!(outcome.queued_backend_commands, 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn refresh_sftp_reassigns_browser_owner_when_current_owner_is_disconnected() {
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

    let outcome = state.apply(Message::RefreshSftp { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        fallback_session_id
    );
    assert!(state.sessions.sftp_browsers[0].loading);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp {
            session_id,
            request,
        }) if *session_id == fallback_session_id && request.remote_path() == "/var/log"
    ));

    state.apply(Message::BackendEventReceived(
        crate::backend::BackendEvent::SftpEntries {
            session_id: fallback_session_id,
            remote_path: "/var/log".to_owned(),
            entries: Vec::new(),
        },
    ));

    assert!(!state.sessions.sftp_browsers[0].loading);
}

#[test]
fn navigate_sftp_message_queues_target_directory_listing() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let outcome = state.apply(Message::NavigateSftp {
        host_id,
        remote_path: "/etc".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/etc"
    ));
}

#[test]
fn save_sftp_bookmark_uses_current_browser_directory() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let outcome = state.apply(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert_eq!(state.storage.sftp_bookmark_count(), 1);
    assert_eq!(state.storage.sftp_bookmarks[0].host_id, host_id);
    assert_eq!(state.storage.sftp_bookmarks[0].label, "ops");
    assert_eq!(state.storage.sftp_bookmarks[0].remote_path, "/home/ops");
}

#[test]
fn save_sftp_bookmark_reports_missing_browser() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::SaveSftpBookmark { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.storage.sftp_bookmark_count(), 0);
}

#[test]
fn open_sftp_bookmark_opens_browser_when_none_exists() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::OpenSftpBookmark {
        host_id,
        remote_path: "/var/log".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/var/log");
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Connect { target, .. }) if target.host_id == host_id
    ));
}

#[test]
fn open_sftp_bookmark_navigates_existing_browser() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let outcome = state.apply(Message::OpenSftpBookmark {
        host_id,
        remote_path: "/etc".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(state.sessions.sftp_browsers[0].loading);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. }) if request.remote_path() == "/etc"
    ));
}

#[test]
fn open_sftp_bookmark_reopens_disconnected_browser() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let old_session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(old_session_id, crate::model::SessionStatus::Disconnected);

    let outcome = state.apply(Message::OpenSftpBookmark {
        host_id,
        remote_path: "/etc".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 2);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/etc");
    let new_session_id = state.sessions.tabs[1].id;
    assert_ne!(old_session_id, new_session_id);
    assert!(matches!(
        state.backend_commands.drain().as_slice(),
        [
            BackendCommand::Connect {
                session_id: connect_session_id,
                ..
            },
            BackendCommand::Sftp {
                session_id: sftp_session_id,
                request,
            },
        ] if *connect_session_id == new_session_id
            && *sftp_session_id == new_session_id
            && request.remote_path() == "/etc"
    ));
}

#[test]
fn remove_sftp_bookmark_updates_storage_or_reports_missing() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());
    state
        .storage
        .upsert_sftp_bookmark(crate::model::SftpBookmark {
            host_id,
            label: "logs".to_owned(),
            remote_path: "/var/log".to_owned(),
        });

    let remove_outcome = state.apply(Message::RemoveSftpBookmark {
        host_id,
        remote_path: "/var/log".to_owned(),
    });
    let missing_outcome = state.apply(Message::RemoveSftpBookmark {
        host_id,
        remote_path: "/var/log".to_owned(),
    });

    assert!(remove_outcome.changed());
    assert_eq!(state.storage.sftp_bookmark_count(), 0);
    assert!(missing_outcome.changed());
    assert!(missing_outcome.error.is_some());
}

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

#[test]
fn download_sftp_message_queues_transfer_and_download_request() {
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
        value: "C:/tmp/deploy.sh".to_owned(),
    });

    let outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.transfer_count(), 1);
    assert!(matches!(
        &state.sessions.transfers[0].direction,
        crate::model::TransferDirection::Download
    ));
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/deploy.sh"
    ));
}

#[test]
fn download_sftp_message_keeps_browser_loading_unchanged() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    assert!(
        state
            .sessions
            .set_sftp_entries(host_id, "/home/ops", Vec::new())
    );
    assert!(!state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(!state.sessions.sftp_browsers[0].loading);
}

#[test]
fn download_sftp_invalid_remote_path_does_not_reassign_browser_owner() {
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

    let outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: " / ".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("根目录"));
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        disconnected_owner_id
    );
    assert_eq!(state.sessions.transfer_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn download_sftp_rejects_empty_and_root_remote_paths() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let empty_outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "  ".to_owned(),
    });
    let root_outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: " / ".to_owned(),
    });

    assert!(empty_outcome.changed());
    assert!(root_outcome.changed());
    assert!(
        empty_outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("不能为空")
    );
    assert!(
        root_outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("根目录")
    );
    assert_eq!(state.sessions.transfer_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn cancel_sftp_transfer_cancels_queued_transfer_and_removes_backend_command() {
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
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    let transfer_id = state.sessions.transfers[0].id;

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert!(state.backend_commands.is_empty());
}

#[test]
fn cancel_sftp_transfer_keeps_same_id_command_from_other_session() {
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
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    let transfer_id = state.sessions.transfers[0].id;
    let owner_session_id = state.sessions.transfers[0].session_id;
    let stale_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.backend_commands.push(BackendCommand::Sftp {
        session_id: stale_session_id,
        request: crate::backend::SftpRequest::Download {
            id: transfer_id,
            remote_path: "/stale/deploy.sh".to_owned(),
            local_path: "C:/tmp/stale-deploy.sh".to_owned(),
        },
    });

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp {
            session_id,
            request: crate::backend::SftpRequest::Download { id, .. },
        }) if *session_id == stale_session_id && *id == transfer_id
    ));
    assert_ne!(owner_session_id, stale_session_id);
}

#[test]
fn cancel_sftp_transfer_rejects_ambiguous_same_id_tasks() {
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
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    let transfer_id = state.sessions.transfers[0].id;
    let current_session_id = state.sessions.transfers[0].session_id;
    let other_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let mut other_task = state.sessions.transfers[0].clone();
    other_task.session_id = other_session_id;
    other_task.local_path = "C:/tmp/other-deploy.sh".to_owned();
    state.sessions.transfers.push(other_task);

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("不唯一"));
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert_eq!(state.sessions.transfers[0].session_id, current_session_id);
    assert_eq!(state.sessions.transfers[1].session_id, other_session_id);
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Queued
    ));
    assert!(matches!(
        state.sessions.transfers[1].status,
        crate::model::TransferStatus::Queued
    ));
}

#[test]
fn cancel_sftp_upload_clears_browser_loading_when_queued_request_is_removed() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
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
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    assert!(state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
}

#[test]
fn cancel_stale_sftp_upload_keeps_current_browser_loading() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
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
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    let stale_session_id = state.sessions.transfers[0].session_id;
    state.backend_commands.drain();
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let current_session_id = state.sessions.sftp_browsers[0].session_id;
    state.backend_commands.push(BackendCommand::Sftp {
        session_id: stale_session_id,
        request: crate::backend::SftpRequest::Upload {
            id: transfer_id,
            local_path: "C:/tmp/app.tar.gz".to_owned(),
            remote_path: "/home/ops/app.tar.gz".to_owned(),
        },
    });
    assert_ne!(stale_session_id, current_session_id);
    assert!(state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(state.sessions.sftp_browsers[0].loading);
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        current_session_id
    );
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
}

#[test]
fn cancel_sftp_upload_keeps_browser_loading_when_another_refresh_request_remains() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
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
    state.apply(Message::UploadSftp { host_id });
    let first_transfer_id = state.sessions.transfers[0].id;
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/assets.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    assert_eq!(state.backend_commands.pending_count(), 2);

    let outcome = state.apply(Message::CancelSftpTransfer {
        transfer_id: first_transfer_id,
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(state.sessions.sftp_browsers[0].loading);
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert!(matches!(
        state.sessions.transfers[1].status,
        crate::model::TransferStatus::Queued
    ));
}

#[test]
fn cancel_sftp_upload_ignores_stale_session_refresh_when_clearing_loading() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let stale_session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let current_session_id = state.sessions.tabs[1].id;
    state.backend_commands.drain();
    assert_ne!(stale_session_id, current_session_id);
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        current_session_id
    );
    assert!(state.sessions.set_sftp_entries_for_session(
        current_session_id,
        "/var/log",
        Vec::new()
    ));
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    state.backend_commands.push(BackendCommand::Sftp {
        session_id: stale_session_id,
        request: crate::backend::SftpRequest::ListDir {
            remote_path: "/home/ops".to_owned(),
        },
    });
    assert!(state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        current_session_id
    );
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp {
            session_id,
            request: crate::backend::SftpRequest::ListDir { .. },
        }) if *session_id == stale_session_id
    ));
}

#[test]
fn cancel_sftp_transfer_rejects_transfer_already_removed_from_queue() {
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
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    let transfer_id = state.sessions.transfers[0].id;
    state.backend_commands.drain();

    let outcome = state.apply(Message::CancelSftpTransfer { transfer_id });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("已经开始"));
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Queued
    ));
}

#[test]
fn create_and_remove_sftp_actions_queue_path_requests() {
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
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "incoming".to_owned(),
    });

    let mkdir_outcome = state.apply(Message::CreateSftpDir { host_id });
    assert!(mkdir_outcome.changed());
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/incoming"
    ));

    state.backend_commands.drain();
    let remove_outcome = state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(remove_outcome.changed());
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/deploy.sh"
    ));
}

#[test]
fn remove_sftp_file_rejects_empty_and_root_paths() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let empty_outcome = state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: "  ".to_owned(),
    });
    let root_outcome = state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: " / ".to_owned(),
    });

    assert!(empty_outcome.changed());
    assert!(root_outcome.changed());
    assert!(
        empty_outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("不能为空")
    );
    assert!(
        root_outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("根目录")
    );
    assert!(state.backend_commands.is_empty());
}

#[test]
fn create_sftp_dir_rejects_path_like_names() {
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
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "releases/2026".to_owned(),
    });

    let outcome = state.apply(Message::CreateSftpDir { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("路径分隔符")
    );
    assert!(state.backend_commands.is_empty());
}

#[test]
fn create_sftp_dir_rejects_parent_directory_alias() {
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
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "..".to_owned(),
    });

    let outcome = state.apply(Message::CreateSftpDir { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("路径分隔符")
    );
    assert!(state.backend_commands.is_empty());
}
