use super::*;
use crate::app::state::{DesktopAppState, DesktopStateView};
use crate::core::CoreState;
use crate::model::{
    AgentSource, AuthProfile, CommandHistoryId, CommandHistoryItem, Host, HostId, LanguageMode,
    RecentConnection, UiState,
};
use uuid::Uuid;

fn desktop_state() -> DesktopAppState {
    let core = CoreState::default();
    let ui = UiState::from_visual(&core.config.theme, &core.config.background);
    DesktopAppState { core, ui }
}

fn view(state: &DesktopAppState) -> DesktopStateView<'_> {
    state.as_desktop_state_view()
}

fn host(name: &str, address: &str, tags: &[&str]) -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: name.to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        address: address.to_owned(),
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

#[test]
fn command_palette_includes_matching_hosts_recent_and_history() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;
    let prod = host("Production API", "api.example.com", &["prod", "linux"]);
    let prod_id = prod.id;
    state.core.storage.upsert_host(prod);
    state
        .core
        .storage
        .upsert_host(host("Jump Box", "jump.internal", &["bastion"]));
    state
        .core
        .storage
        .record_recent_connection(RecentConnection {
            host_id: prod_id,
            label: "Production API".to_owned(),
            connected_at_unix_secs: 1,
        });
    state.core.storage.add_command_history(CommandHistoryItem {
        id: CommandHistoryId(Uuid::new_v4()),
        host_id: Some(prod_id),
        command: "systemctl status api".to_owned(),
        working_directory: None,
        exit_code: Some(0),
        started_at_unix_secs: 2,
        duration_ms: Some(30),
    });
    state.ui.workspace.open_command_palette("production");

    let rows = command_palette_results(view(&state));

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row.kind_key == "Host"));
    assert!(rows.iter().any(|row| row.kind == "Host"));
    assert!(rows.iter().any(|row| row.kind_key == "Recent"));
    assert!(rows.iter().any(|row| row.kind == "Recent"));
}

#[test]
fn command_palette_keeps_logic_kind_stable_when_localized() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::Chinese;
    state
        .core
        .storage
        .upsert_host(host("Production API", "api.example.com", &["prod"]));
    state.ui.workspace.open_command_palette("production");

    let rows = command_palette_results(view(&state));

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind_key, "Host");
    assert_eq!(rows[0].kind, "主机");
}

#[test]
fn command_palette_searches_history_commands() {
    let mut state = desktop_state();
    state.ui.workspace.language = LanguageMode::English;
    let prod = host("Production API", "api.example.com", &["prod"]);
    let prod_id = prod.id;
    state.core.storage.upsert_host(prod);
    state.core.storage.add_command_history(CommandHistoryItem {
        id: CommandHistoryId(Uuid::new_v4()),
        host_id: Some(prod_id),
        command: "journalctl -u api".to_owned(),
        working_directory: None,
        exit_code: Some(0),
        started_at_unix_secs: 2,
        duration_ms: Some(30),
    });
    state.ui.workspace.open_command_palette("journal");

    let rows = command_palette_results(view(&state));

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "History");
    assert_eq!(rows[0].subtitle, "history · Production API");
}
