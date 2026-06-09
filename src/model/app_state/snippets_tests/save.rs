use super::*;

#[test]
fn save_host_command_snippet_uses_current_command_draft() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state
        .ui
        .set_remote_command(host_id, "  systemctl status sshd  ");

    let outcome = state.apply(Message::SaveHostCommandSnippet { host_id });

    assert!(outcome.changed());
    assert_eq!(state.storage.snippet_count(), 1);
    assert_eq!(state.storage.snippets[0].name, "systemctl status sshd");
    assert_eq!(
        state.storage.snippets[0].default_command_template(),
        "systemctl status sshd"
    );
    assert_eq!(state.storage.snippets[0].scope, SnippetScope::Host(host_id));
}

#[test]
fn save_host_command_snippet_infers_template_variables() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state
        .ui
        .set_remote_command(host_id, "systemctl restart {{service}}");

    let outcome = state.apply(Message::SaveHostCommandSnippet { host_id });

    assert!(outcome.changed());
    assert_eq!(state.storage.snippets[0].variables.len(), 1);
    assert_eq!(state.storage.snippets[0].variables[0].name, "service");
    assert!(state.storage.snippets[0].variables[0].required);
}

#[test]
fn save_host_command_snippet_rejects_empty_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.ui.set_remote_command(host_id, "   ");

    let outcome = state.apply(Message::SaveHostCommandSnippet { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.storage.snippet_count(), 0);
}
