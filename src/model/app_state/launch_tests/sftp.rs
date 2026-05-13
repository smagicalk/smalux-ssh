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
