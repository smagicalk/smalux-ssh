use super::super::AppState;
use crate::config::{BuiltInThemePreference, HostListModePreference, LanguagePreference};
use crate::model::{
    BuiltInTheme, HostId, LanguageMode, Message, SessionId, ToolPanelMode, WorkspacePage,
};
use crate::terminal::TerminalTabState;
use uuid::Uuid;

#[test]
fn workspace_ui_messages_update_layout_state_only() {
    let mut state = AppState::default();

    state.apply(Message::SetWorkspacePage {
        page: WorkspacePage::Settings,
    });
    state.apply(Message::ToggleHostListMode);
    state.apply(Message::UpdateHostSearchQuery {
        query: "prod".to_owned(),
    });
    state.apply(Message::UpdateNetworkSearchQuery {
        query: "proxy".to_owned(),
    });
    state.apply(Message::ResizeHostsPanel { width: 260 });
    state.apply(Message::ResizeActivityPanel { width: 300 });
    state.apply(Message::ResizeToolPanel { width: 360 });
    state.apply(Message::OpenToolPanel {
        mode: ToolPanelMode::History,
    });
    state.apply(Message::ToggleRightSidebar);
    state.apply(Message::OpenCommandPalette {
        query: "prod".to_owned(),
    });
    state.apply(Message::OpenCreateHostDialog);

    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Settings);
    assert!(matches!(
        state.ui.workspace.host_list_mode,
        crate::model::HostListMode::Card
    ));
    assert_eq!(
        state.config.workspace.host_list_mode,
        HostListModePreference::Card
    );
    assert_eq!(
        state.storage.app_config.workspace.host_list_mode,
        HostListModePreference::Card
    );
    assert_eq!(state.ui.workspace.host_search_query, "prod");
    assert_eq!(state.ui.workspace.network_search_query, "proxy");
    assert_eq!(state.ui.workspace.hosts_panel_width, 260);
    assert_eq!(state.ui.workspace.activity_panel_width, 300);
    assert_eq!(state.ui.workspace.tool_panel_width, 360);
    assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::History);
    assert!(state.ui.workspace.right_sidebar_collapsed);
    assert!(state.ui.workspace.command_palette.open);
    assert_eq!(state.ui.workspace.command_palette.query, "prod");
    assert!(state.ui.workspace.create_host_dialog_open);
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn repeated_navigation_toggles_hosts_panel_collapse() {
    let mut state = AppState::default();

    let collapse = state.apply(Message::NavigateWorkspacePage {
        page: WorkspacePage::Hosts,
    });
    let expand = state.apply(Message::NavigateWorkspacePage {
        page: WorkspacePage::Hosts,
    });
    state.apply(Message::NavigateWorkspacePage {
        page: WorkspacePage::Terminal,
    });

    assert!(collapse.changed());
    assert!(expand.changed());
    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Terminal);
    assert!(!state.ui.workspace.hosts_panel_collapsed);
}

#[test]
fn settings_messages_update_workspace_preferences() {
    let mut state = AppState::default();

    let language = state.apply(Message::SetLanguage {
        language: LanguageMode::Chinese,
    });
    let theme = state.apply(Message::SetBuiltInTheme {
        theme: BuiltInTheme::SolarizedDark,
    });

    assert!(language.changed());
    assert!(theme.changed());
    assert_eq!(state.ui.workspace.language, LanguageMode::Chinese);
    assert_eq!(state.config.workspace.language, LanguagePreference::Chinese);
    assert_eq!(state.ui.workspace.theme, BuiltInTheme::SolarizedDark);
    assert_eq!(
        state.config.workspace.built_in_theme,
        BuiltInThemePreference::SolarizedDark
    );
    assert_eq!(state.storage.app_config, state.config);
}

#[test]
fn create_host_dialog_messages_update_ui_state_only() {
    let mut state = AppState::default();
    state.ui.quick_host.name = "stale".to_owned();

    let open = state.apply(Message::OpenCreateHostDialog);
    let close = state.apply(Message::CloseCreateHostDialog);

    assert!(open.changed());
    assert!(close.changed());
    assert!(!state.ui.workspace.create_host_dialog_open);
    assert_eq!(state.ui.quick_host.name, "");
    assert_eq!(state.storage.host_count(), 0);
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn tool_panel_messages_update_layout_state_only() {
    let mut state = AppState::default();

    let open = state.apply(Message::OpenToolPanel {
        mode: ToolPanelMode::KnownHosts,
    });
    let close = state.apply(Message::CloseToolPanel);

    assert!(open.changed());
    assert!(close.changed());
    assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::Closed);
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn opening_sftp_tool_panel_returns_to_terminal_workspace() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());
    let session_id = SessionId(Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .terminal
        .open_tab(TerminalTabState::new(session_id, "production"));
    state.ui.workspace.active_page = WorkspacePage::Sftp;

    let open = state.apply(Message::OpenToolPanel {
        mode: ToolPanelMode::Sftp,
    });

    assert!(open.changed());
    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Terminal);
    assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::Sftp);
    assert_eq!(state.sessions.active_tab, Some(session_id));
}
