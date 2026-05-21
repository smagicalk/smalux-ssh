use super::*;

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
