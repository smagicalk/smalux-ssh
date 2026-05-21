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
