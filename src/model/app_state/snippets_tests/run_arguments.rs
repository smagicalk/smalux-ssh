use super::*;

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
