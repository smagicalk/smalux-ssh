use super::*;
use crate::backend::BackendCommand;
use crate::model::{
    AuthProfile, Host, SecretRef, Snippet, SnippetArgument, SnippetScope, SnippetVariable,
};
use uuid::Uuid;

fn sample_host() -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: "staging".to_owned(),
        group_id: None,
        tags: vec!["linux".to_owned()],
        address: "staging.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Password {
            username: "ops".to_owned(),
            secret: SecretRef("password:ops".to_owned()),
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

fn host_snippet(host_id: HostId, command: &str) -> Snippet {
    Snippet {
        id: SnippetId(Uuid::new_v4()),
        name: command.to_owned(),
        description: None,
        command_template: command.to_owned(),
        scope: SnippetScope::Host(host_id),
        variables: Vec::new(),
        last_arguments: Vec::new(),
    }
}

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
        state.storage.snippets[0].command_template,
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
