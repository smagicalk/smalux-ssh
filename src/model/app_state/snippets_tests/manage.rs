use super::*;

#[test]
fn update_snippet_argument_reports_unknown_variable() {
    let mut state = AppState::default();
    let snippet_id = SnippetId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateSnippetArgument {
        snippet_id,
        name: "service".to_owned(),
        value: "nginx".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
}

#[test]
fn remove_snippet_deletes_existing_snippet_and_reports_missing() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());
    let snippet = host_snippet(host_id, "uptime");
    let snippet_id = snippet.id;
    state.storage.upsert_snippet(snippet);

    let removed = state.apply(Message::RemoveSnippet { snippet_id });
    let missing = state.apply(Message::RemoveSnippet { snippet_id });

    assert!(removed.changed());
    assert_eq!(state.storage.snippet_count(), 0);
    assert!(missing.changed());
    assert!(missing.error.is_some());
}
