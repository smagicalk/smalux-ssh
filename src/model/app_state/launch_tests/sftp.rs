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
