use super::*;

#[test]
fn run_snippet_reports_missing_variable_until_arguments_exist() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = parameterized_host_snippet(host_id, "systemctl restart {{service}}");
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
fn update_snippet_argument_allows_parameterized_snippet_to_run() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = parameterized_host_snippet(host_id, "systemctl restart {{service}}");
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
        snippet_arguments(&state.storage.snippets[0]),
        vec![SnippetArgument {
            name: "service".to_owned(),
            value: "nginx".to_owned(),
        }]
        .as_slice()
    );

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[1],
        BackendCommand::RunCommand { request, .. }
            if request.command == "systemctl restart nginx"
    ));
}

#[test]
fn run_snippet_with_arguments_records_arguments_and_runs() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = parameterized_host_snippet(host_id, "systemctl restart {{service}}");
    let snippet_id = snippet.id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RunSnippetWithArguments {
        host_id,
        snippet_id,
        arguments: vec![
            SnippetArgument {
                name: "service".to_owned(),
                value: "nginx".to_owned(),
            },
            SnippetArgument {
                name: "unused".to_owned(),
                value: "ignored".to_owned(),
            },
        ],
    });

    assert!(outcome.changed());
    assert_eq!(
        snippet_arguments(&state.storage.snippets[0]),
        vec![SnippetArgument {
            name: "service".to_owned(),
            value: "nginx".to_owned(),
        }]
        .as_slice()
    );

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[1],
        BackendCommand::RunCommand { request, .. }
            if request.command == "systemctl restart nginx"
    ));
}

#[test]
fn run_snippet_with_arguments_does_not_record_invalid_run() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    let snippet = parameterized_host_snippet(host_id, "systemctl restart {{service}}");
    let snippet_id = snippet.id;
    state.storage.upsert_host(host);
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RunSnippetWithArguments {
        host_id,
        snippet_id,
        arguments: Vec::new(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.error.as_deref(), Some("快捷命令缺少变量：service"));
    assert!(snippet_arguments(&state.storage.snippets[0]).is_empty());
    assert!(state.backend_commands.is_empty());
}
