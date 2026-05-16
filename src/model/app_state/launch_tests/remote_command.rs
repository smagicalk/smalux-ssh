use super::*;

#[test]
fn run_remote_command_queues_exec_request_and_records_history() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::RunRemoteCommand {
        host_id,
        command: " uptime ".to_owned(),
        request_pty: false,
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.terminal.tab_count(), 1);
    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Terminal);
    assert_eq!(state.storage.recent_count(), 1);
    assert_eq!(state.storage.command_history_count(), 1);
    assert_eq!(state.storage.command_history[0].command, "uptime");
    assert_eq!(state.storage.command_history[0].host_id, Some(host_id));
    assert!(state.storage.command_history[0].started_at_unix_secs > 0);
    assert!(matches!(
        &state.sessions.tabs[0].kind,
        SessionKind::RemoteCommand { command } if command == "uptime"
    ));

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
        BackendCommand::RunCommand {
            session_id: command_session_id,
            request,
        } if *command_session_id == session_id
            && request.command == "uptime"
            && request.pty.is_none()
    ));
}

#[test]
fn run_remote_command_can_request_pty() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::RunRemoteCommand {
        host_id,
        command: "top".to_owned(),
        request_pty: true,
    });

    let commands = state.backend_commands.drain();

    assert!(matches!(
        &commands[1],
        BackendCommand::RunCommand { request, .. }
            if request.command == "top" && request.pty.is_some()
    ));
}

#[test]
fn run_remote_command_rejects_empty_command_without_side_effects() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::RunRemoteCommand {
        host_id,
        command: "   ".to_owned(),
        request_pty: false,
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert_eq!(state.storage.command_history_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn run_remote_command_reports_missing_host_without_history() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::RunRemoteCommand {
        host_id,
        command: "uptime".to_owned(),
        request_pty: false,
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.storage.command_history_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn run_command_history_reuses_recorded_host_and_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.record_command_history(host_id, "df -h".to_owned());
    let history_id = state.storage.command_history[0].id;

    let outcome = state.apply(Message::RunCommandHistory { history_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.terminal.tab_count(), 1);
    assert_eq!(state.storage.command_history_count(), 2);
    assert_eq!(state.storage.command_history[1].command, "df -h");

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[1],
        BackendCommand::RunCommand { request, .. }
            if request.command == "df -h" && request.pty.is_none()
    ));
}

#[test]
fn run_command_history_reports_missing_history() {
    let mut state = AppState::default();
    let history_id = crate::model::CommandHistoryId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::RunCommandHistory { history_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert!(state.backend_commands.is_empty());
}

#[test]
fn run_command_history_rejects_global_history_without_host() {
    let mut state = AppState::default();
    let history_id = crate::model::CommandHistoryId(uuid::Uuid::new_v4());
    state
        .storage
        .add_command_history(crate::model::CommandHistoryItem {
            id: history_id,
            host_id: None,
            command: "uptime".to_owned(),
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: 1,
            duration_ms: None,
        });

    let outcome = state.apply(Message::RunCommandHistory { history_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.storage.command_history_count(), 1);
    assert!(state.backend_commands.is_empty());
}
