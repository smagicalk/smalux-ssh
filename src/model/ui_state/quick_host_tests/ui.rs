use crate::model::{
    AuthProfile, Host, HostId, QuickHostAgentSource, QuickHostAuthField, QuickHostAuthKind,
    QuickHostDraftField, SecretRef, UiState,
};
use uuid::Uuid;

#[test]
fn ui_state_quick_host_messages_update_form_only() {
    let mut state = UiState::default();

    state.set_quick_host_field(QuickHostDraftField::Address, "example.com");
    state.set_quick_host_field(QuickHostDraftField::Username, "ops");
    state.set_quick_host_auth_kind(QuickHostAuthKind::Password);
    state.set_quick_host_auth_field(QuickHostAuthField::PasswordSecretRef, "password:ops");
    state.reset_quick_host();

    assert_eq!(state.quick_host.address, "");
    assert_eq!(state.quick_host.username, "");
    assert_eq!(state.quick_host.port, "22");
    assert!(matches!(
        state.quick_host.auth.kind,
        QuickHostAuthKind::Agent
    ));
    assert_eq!(
        state.quick_host.auth.agent_source,
        QuickHostAgentSource::Auto
    );
}

#[test]
fn ui_state_updates_quick_host_agent_source() {
    let mut state = UiState::default();

    state.set_quick_host_auth_field(QuickHostAuthField::AgentSource, "Pageant");
    state.set_quick_host_auth_field(
        QuickHostAuthField::AgentCustomPipe,
        r"\\.\pipe\custom-agent",
    );

    assert_eq!(
        state.quick_host.auth.agent_source,
        QuickHostAgentSource::Pageant
    );
    assert_eq!(
        state.quick_host.auth.agent_custom_pipe,
        r"\\.\pipe\custom-agent"
    );
}

#[test]
fn ui_state_can_prefill_quick_host_for_editing() {
    let mut state = UiState::default();
    let host_id = HostId(Uuid::new_v4());

    state.edit_quick_host(&Host {
        id: host_id,
        name: "prod".to_owned(),
        group_id: None,
        icon_key: "database".to_owned(),
        tags: vec!["prod".to_owned(), "linux".to_owned()],
        address: "prod.example.com".to_owned(),
        port: 2202,
        auth: AuthProfile::Password {
            username: "root".to_owned(),
            secret: SecretRef("password:root".to_owned()),
        },
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });

    assert_eq!(state.quick_host.editing_host_id, Some(host_id));
    assert_eq!(state.quick_host.name, "prod");
    assert_eq!(state.quick_host.address, "prod.example.com");
    assert_eq!(state.quick_host.port, "2202");
    assert_eq!(state.quick_host.username, "root");
    assert_eq!(state.quick_host.icon_key, "database");
    assert_eq!(state.quick_host.tags, "prod, linux");
    assert!(matches!(
        state.quick_host.auth.kind,
        QuickHostAuthKind::Password
    ));
    assert_eq!(state.quick_host.auth.password_secret_ref, "password:root");
}
