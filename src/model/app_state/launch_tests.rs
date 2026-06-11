use super::*;
use crate::backend::BackendCommand;
use crate::model::{AgentSource, AuthProfile, Host, SecretRef, SessionKind, SessionStatus};

mod remote_command;
mod sftp;
mod shell;
mod tunnel;

fn sample_host() -> Host {
    Host {
        id: HostId(uuid::Uuid::new_v4()),
        name: "production".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["prod".to_owned()],
        address: "example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            source: AgentSource::Auto,
            key_hint: Some("id_ed25519".to_owned()),
        },
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}
