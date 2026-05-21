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
        SessionKind::RemoteCommand {
            command,
            history_id,
        } if command == "uptime"
            && *history_id == Some(state.storage.command_history[0].id)
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
