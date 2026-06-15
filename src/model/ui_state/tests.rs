use super::*;
use crate::config::{
    AppConfig, BuiltInThemePreference, HostListModePreference, LanguagePreference,
};
use uuid::Uuid;

fn host_id() -> HostId {
    HostId(Uuid::new_v4())
}

#[test]
fn default_values_are_actionable_without_saved_draft() {
    let ui = UiState::default();
    let host_id = host_id();

    assert_eq!(ui.remote_command_for(host_id), "uptime");
    assert_eq!(ui.sftp_initial_dir_for(host_id), "/");
    assert!(ui.last_error.is_none());
}

#[test]
fn last_error_can_be_set_and_cleared() {
    let mut ui = UiState::default();

    assert!(ui.set_last_error("连接失败"));
    assert_eq!(ui.last_error.as_deref(), Some("连接失败"));
    assert!(!ui.set_last_error("连接失败"));
    assert!(ui.clear_last_error());
    assert!(ui.last_error.is_none());
    assert!(!ui.clear_last_error());
}

#[test]
fn host_action_drafts_are_scoped_per_host() {
    let mut ui = UiState::default();
    let first = host_id();
    let second = host_id();

    ui.set_remote_command(first, "df -h");
    ui.set_sftp_initial_dir(second, "/var/log");

    assert_eq!(ui.remote_command_for(first), "df -h");
    assert_eq!(ui.sftp_initial_dir_for(first), "/");
    assert_eq!(ui.remote_command_for(second), "uptime");
    assert_eq!(ui.sftp_initial_dir_for(second), "/var/log");
    assert_eq!(ui.host_action_drafts.len(), 2);
}

#[test]
fn workspace_preferences_can_be_applied_from_config() {
    let mut ui = UiState::default();
    let mut config = AppConfig::default();
    config.workspace.host_list_mode = HostListModePreference::Card;
    config.workspace.language = LanguagePreference::English;
    config.workspace.built_in_theme = BuiltInThemePreference::Dracula;

    ui.apply_workspace_preferences_from_config(&config);

    assert_eq!(ui.workspace.host_list_mode, HostListMode::Card);
    assert_eq!(ui.workspace.language, LanguageMode::English);
    assert_eq!(ui.workspace.theme, BuiltInTheme::Dracula);
}
