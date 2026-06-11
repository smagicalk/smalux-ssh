use super::*;
use crate::backend::BackendCommand;
use crate::model::{
    AuthProfile, Host, SecretRef, Snippet, SnippetArgument, SnippetImplementation,
    SnippetImplementationId, SnippetScope, SnippetShell, SnippetSupportTarget,
    SnippetSupportTargetId,
};
use uuid::Uuid;

mod manage;
mod run;
mod save;

fn sample_host() -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: "staging".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["linux".to_owned()],
        address: "staging.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Password {
            username: "ops".to_owned(),
            secret: SecretRef("password:ops".to_owned()),
        },
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

fn host_snippet(host_id: HostId, command: &str) -> Snippet {
    Snippet::with_default_implementation(
        SnippetId(Uuid::new_v4()),
        command.to_owned(),
        None,
        SnippetScope::Host(host_id),
        None,
        command.to_owned(),
    )
}

fn parameterized_host_snippet(host_id: HostId, command: &str) -> Snippet {
    Snippet::with_default_implementation(
        SnippetId(Uuid::new_v4()),
        "restart".to_owned(),
        None,
        SnippetScope::Host(host_id),
        None,
        command.to_owned(),
    )
}

fn snippet_arguments(snippet: &Snippet) -> &[SnippetArgument] {
    snippet
        .default_implementation()
        .expect("默认实现应存在")
        .last_arguments
        .as_slice()
}

fn multi_target_snippet(host_id: HostId) -> Snippet {
    let mut snippet = Snippet::with_default_implementation(
        SnippetId(Uuid::new_v4()),
        "list".to_owned(),
        None,
        SnippetScope::Host(host_id),
        None,
        "ls {{path}}".to_owned(),
    );
    let windows_implementation_id = SnippetImplementationId(Uuid::new_v4());
    snippet.implementations.push(SnippetImplementation {
        id: windows_implementation_id,
        snippet_id: snippet.id,
        name: "Windows 脚本".to_owned(),
        shell: SnippetShell::PowerShell,
        command_template: "dir {{path}}".to_owned(),
        notes: None,
        last_arguments: Vec::new(),
        sort_order: 1,
    });
    snippet.support_targets.push(SnippetSupportTarget {
        id: SnippetSupportTargetId(Uuid::new_v4()),
        snippet_id: snippet.id,
        target_key: "windows".to_owned(),
        display_name: "Windows".to_owned(),
        implementation_id: windows_implementation_id,
        sort_order: 1,
    });
    snippet
}
