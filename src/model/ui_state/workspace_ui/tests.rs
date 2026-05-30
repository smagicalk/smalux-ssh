use super::*;

#[test]
fn workspace_ui_defaults_to_hosts_follow_system_language() {
    let state = WorkspaceUiState::default();

    assert_eq!(state.active_page, WorkspacePage::Hosts);
    assert_eq!(state.language, LanguageMode::FollowSystem);
    assert_eq!(state.host_list_mode, HostListMode::Tree);
    assert!(state.host_search_query.is_empty());
    assert!(!state.create_host_dialog_open);
    assert_eq!(state.hosts_panel_width, DEFAULT_HOSTS_PANEL_WIDTH);
    assert_eq!(state.activity_panel_width, DEFAULT_ACTIVITY_PANEL_WIDTH);
    assert_eq!(state.tool_panel_width, DEFAULT_TOOL_PANEL_WIDTH);
    assert_eq!(state.tool_panel_mode, ToolPanelMode::Closed);
}

#[test]
fn create_host_dialog_can_open_and_close() {
    let mut state = WorkspaceUiState::default();

    state.create_host_dialog_open = true;
    assert!(state.create_host_dialog_open);

    state.create_host_dialog_open = false;
    assert!(!state.create_host_dialog_open);
}

#[test]
fn command_palette_can_open_and_close() {
    let mut state = WorkspaceUiState::default();

    state.open_command_palette("prod");
    assert!(state.command_palette.open);
    assert_eq!(state.command_palette.query, "prod");

    state.close_command_palette();
    assert!(!state.command_palette.open);
    assert!(state.command_palette.query.is_empty());
}

#[test]
fn host_search_query_is_ui_only_state() {
    let mut state = WorkspaceUiState::default();

    state.set_host_search_query("prod");

    assert_eq!(state.host_search_query, "prod");
}

#[test]
fn panel_widths_are_clamped() {
    let mut state = WorkspaceUiState::default();

    state.set_hosts_panel_width(1);
    state.set_activity_panel_width(9_999);
    state.set_tool_panel_width(9);

    assert_eq!(state.hosts_panel_width, MIN_HOSTS_PANEL_WIDTH);
    assert_eq!(state.activity_panel_width, MAX_ACTIVITY_PANEL_WIDTH);
    assert_eq!(state.tool_panel_width, MIN_TOOL_PANEL_WIDTH);
}

#[test]
fn tool_panel_can_open_and_close_without_changing_width() {
    let mut state = WorkspaceUiState::default();

    state.open_tool_panel(ToolPanelMode::Sftp);
    assert_eq!(state.tool_panel_mode, ToolPanelMode::Sftp);
    assert_eq!(state.tool_panel_width, DEFAULT_TOOL_PANEL_WIDTH);

    state.close_tool_panel();
    assert_eq!(state.tool_panel_mode, ToolPanelMode::Closed);
}

#[test]
fn background_carousel_wraps_index() {
    let mut state = WorkspaceUiState::default();

    state.next_background(2);
    assert_eq!(state.active_background_index(2), Some(1));
    state.next_background(2);
    assert_eq!(state.active_background_index(2), Some(0));
    state.next_background(0);
    assert_eq!(state.active_background_index(0), None);
}

#[test]
fn built_in_theme_cycles_through_configured_palettes() {
    let mut state = WorkspaceUiState::default();

    state.next_theme();
    assert_eq!(state.theme, BuiltInTheme::CatppuccinMocha);

    state.next_theme();
    assert_eq!(state.theme, BuiltInTheme::NordDark);
}
