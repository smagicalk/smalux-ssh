use super::*;
use crate::app::state::DesktopAppState;
use crate::backend::{BackendCommand, BackendEvent};
use crate::core::CoreState;
use crate::model::{
    AgentSource, AuthProfile, Host, SessionStatus, TunnelKind, TunnelRule, TunnelRuntimeState,
    TunnelStatus, UiState,
};

mod activation;
mod base;
mod close_tabs;
mod sftp_selection;
mod terminal_input;

fn core_state() -> CoreState {
    CoreState::default()
}

fn desktop_state() -> DesktopAppState {
    let core = CoreState::default();
    let ui = UiState::from_visual(&core.config.theme, &core.config.background);
    DesktopAppState { core, ui }
}

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
