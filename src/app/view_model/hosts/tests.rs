use super::*;
use crate::model::{AuthProfile, Host, HostId};
use uuid::Uuid;

fn agent_host(name: &str, address: &str, tags: &[&str]) -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: name.to_owned(),
        group_id: None,
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        address: address.to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            key_hint: None,
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

#[test]
fn host_rows_do_not_expose_auth_secrets() {
    let mut state = AppState::default();
    state.storage.upsert_host(Host {
        id: HostId(Uuid::new_v4()),
        name: "root".to_owned(),
        group_id: None,
        tags: Vec::new(),
        address: "example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Password {
            username: "root".to_owned(),
            secret: crate::model::SecretRef("password:root".to_owned()),
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });

    let rows = hosts(&state);

    assert_eq!(rows[0].auth, "Password");
    assert!(!rows[0].endpoint.contains("password"));
}

#[test]
fn host_rows_follow_search_query() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_host(agent_host("Production", "prod.example.com", &["prod"]));
    state
        .storage
        .upsert_host(agent_host("Staging", "staging.example.com", &["stage"]));
    state.ui.workspace.set_host_search_query("prod");

    let rows = hosts(&state);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Production");
}
