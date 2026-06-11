use super::*;
use crate::model::{AgentSource, AuthProfile, Host, HostId, ImageSource, Message};
use uuid::Uuid;

fn sample_host() -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: "production".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "prod.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            source: AgentSource::Auto,
            key_hint: None,
        },
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

#[path = "visual_settings_tests_global.rs"]
mod global;
#[path = "visual_settings_tests_host.rs"]
mod host;
#[path = "visual_settings_tests_validation.rs"]
mod validation;
