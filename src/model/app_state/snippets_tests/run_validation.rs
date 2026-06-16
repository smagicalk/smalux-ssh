use super::*;

#[test]
fn run_snippet_rejects_snippet_from_other_host() {
    let mut state = CoreState::default();
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
fn run_snippet_rejects_empty_rendered_command_without_side_effects() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    let mut snippet = parameterized_host_snippet(host_id, "{{maybe}}");
    snippet.name = "optional".to_owned();
    snippet.variables[0].required = false;
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
fn run_snippet_on_active_host_requires_remote_tab() {
    let mut state = CoreState::default();
    let snippet = host_snippet(HostId(Uuid::new_v4()), "uptime");
    let snippet_id = snippet.id;
    state.storage.upsert_snippet(snippet);

    let outcome = state.apply(Message::RunSnippetOnActiveHost { snippet_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert!(state.backend_commands.is_empty());
}
