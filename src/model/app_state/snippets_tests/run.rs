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
fn run_snippet_rejects_snippet_from_other_host() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let other_host_id = HostId(Uuid::new_v4());
    let snippet = host_snippet(other_host_id, "uptime");
    let snippet_id = snippet.id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RunSnippet {
        host_id,
        snippet_id,
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.command_history_count(), 0);
}

#[test]
fn run_snippet_reports_missing_variable_until_arguments_exist() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = Snippet {
        id: SnippetId(Uuid::new_v4()),
        name: "restart".to_owned(),
        description: None,
        command_template: "systemctl restart {{service}}".to_owned(),
        scope: SnippetScope::Host(host_id),
        variables: vec![SnippetVariable {
            name: "service".to_owned(),
            default_value: None,
            required: true,
        }],
        last_arguments: Vec::new(),
    };
    let snippet_id = snippet.id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RunSnippet {
        host_id,
        snippet_id,
    });

    assert!(outcome.changed());
    assert_eq!(outcome.error.as_deref(), Some("快捷命令缺少变量：service"));
    assert!(state.backend_commands.is_empty());
}

#[test]
fn run_snippet_rejects_empty_rendered_command_without_side_effects() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = Snippet {
        id: SnippetId(Uuid::new_v4()),
        name: "optional".to_owned(),
        description: None,
        command_template: "{{maybe}}".to_owned(),
        scope: SnippetScope::Host(host_id),
        variables: vec![SnippetVariable {
            name: "maybe".to_owned(),
            default_value: None,
            required: false,
        }],
        last_arguments: Vec::new(),
    };
    let snippet_id = snippet.id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RunSnippet {
        host_id,
        snippet_id,
    });

    assert!(outcome.changed());
    assert_eq!(outcome.error.as_deref(), Some("快捷命令渲染结果不能为空"));
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert_eq!(state.storage.command_history_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn update_snippet_argument_allows_parameterized_snippet_to_run() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = Snippet {
        id: SnippetId(Uuid::new_v4()),
        name: "restart".to_owned(),
        description: None,
        command_template: "systemctl restart {{service}}".to_owned(),
        scope: SnippetScope::Host(host_id),
        variables: vec![SnippetVariable {
            name: "service".to_owned(),
            default_value: None,
            required: true,
        }],
        last_arguments: Vec::new(),
    };
    let snippet_id = snippet.id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);

    let updated = state.apply(Message::UpdateSnippetArgument {
        snippet_id,
        name: "service".to_owned(),
        value: "nginx".to_owned(),
    });
    let outcome = state.apply(Message::RunSnippet {
        host_id,
        snippet_id,
    });

    assert!(updated.changed());
    assert!(outcome.changed());
    assert_eq!(
        state.storage.snippets[0].last_arguments,
        vec![SnippetArgument {
            name: "service".to_owned(),
            value: "nginx".to_owned(),
        }]
    );

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[1],
        BackendCommand::RunCommand { request, .. }
            if request.command == "systemctl restart nginx"
    ));
}
