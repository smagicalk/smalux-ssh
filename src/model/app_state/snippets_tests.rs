use super::*;
use crate::backend::BackendCommand;
use crate::model::{
    AuthProfile, Host, SecretRef, Snippet, SnippetArgument, SnippetScope, SnippetVariable,
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
