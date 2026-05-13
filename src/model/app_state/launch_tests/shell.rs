use super::*;

#[test]
fn open_shell_message_creates_tabs_and_queues_backend_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::OpenShell { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.terminal.tab_count(), 1);
    assert_eq!(state.storage.recent_count(), 1);
    assert_eq!(state.storage.recent_connections[0].label, "production");
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connecting
    ));
    assert_eq!(state.backend_commands.pending_count(), 2);

    let commands = state.backend_commands.drain();
    let session_id = state.sessions.tabs[0].id;
    assert!(matches!(
        &commands[0],
        BackendCommand::Connect {
            session_id: command_session_id,
            target,
        } if *command_session_id == session_id
            && target.host_id == host_id
            && target.endpoint() == "example.com:22"
    ));
    assert!(matches!(
        &commands[1],
        BackendCommand::OpenShell {
            session_id: command_session_id,
            pty,
        } if *command_session_id == session_id && pty.term == "xterm-256color"
    ));
}

#[test]
fn open_shell_message_reports_missing_host_without_queueing_commands() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::OpenShell { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.sessions.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn open_recent_connection_reuses_shell_launch_flow() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state
        .storage
        .record_recent_connection(crate::model::RecentConnection {
            host_id,
            label: "production".to_owned(),
            connected_at_unix_secs: 1,
        });

    let outcome = state.apply(Message::OpenRecentConnection { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.terminal.tab_count(), 1);
    assert_eq!(state.storage.recent_count(), 1);
    assert_eq!(state.storage.recent_connections[0].label, "production");
    assert_eq!(state.backend_commands.pending_count(), 2);
}

#[test]
fn open_recent_connection_reports_deleted_host() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::OpenRecentConnection { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.sessions.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn password_host_can_still_open_shell_without_exposing_secret() {
    let mut state = AppState::default();
    let mut host = sample_host();
    host.auth = AuthProfile::Password {
        username: "root".to_owned(),
        secret: SecretRef("password:root".to_owned()),
    };
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenShell { host_id });

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[0],
        BackendCommand::Connect { target, .. }
            if target.auth.username() == "root"
    ));
}
