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

#[test]
fn run_snippet_on_active_host_uses_current_remote_tab() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = host_snippet(host_id, "uptime");
    let snippet_id = snippet.id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);
    state.apply(Message::OpenShell { host_id });
    state.backend_commands.drain();

    let outcome = state.apply(Message::RunSnippetOnActiveHost { snippet_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.storage.command_history[0].command, "uptime");
}

#[test]
fn run_snippet_target_uses_target_implementation() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = multi_target_snippet(host_id);
    let snippet_id = snippet.id;
    let windows_target_id = snippet.support_targets[1].id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RunSnippetTargetWithArguments {
        host_id,
        snippet_id,
        target_id: windows_target_id,
        arguments: vec![SnippetArgument {
            name: "path".to_owned(),
            value: "C:\\Temp".to_owned(),
        }],
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.command_history[0].command, "dir C:\\Temp");
    assert!(
        state.storage.snippets[0].implementations[0]
            .last_arguments
            .is_empty()
    );
    assert_eq!(
        state.storage.snippets[0].implementations[1].last_arguments,
        vec![SnippetArgument {
            name: "path".to_owned(),
            value: "C:\\Temp".to_owned(),
        }]
    );
    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[1],
        BackendCommand::RunCommand { request, .. }
            if request.command == "dir C:\\Temp"
    ));
}
