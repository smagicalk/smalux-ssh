use super::*;

#[test]
fn run_snippet_renders_and_runs_remote_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = host_snippet(host_id, "df -h");
    let snippet_id = snippet.id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RunSnippet {
        host_id,
        snippet_id,
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.storage.command_history_count(), 1);
    assert_eq!(state.storage.command_history[0].command, "df -h");

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[1],
        BackendCommand::RunCommand { request, .. }
            if request.command == "df -h" && request.pty.is_none()
    ));
}
