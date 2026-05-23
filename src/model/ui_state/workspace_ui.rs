//! 桌面工作区自身的 UI 状态。

use serde::{Deserialize, Serialize};

#[path = "workspace_ui/background.rs"]
mod background;
#[path = "workspace_ui/command_palette.rs"]
mod command_palette;
#[path = "workspace_ui/language.rs"]
mod language;
#[path = "workspace_ui/layout.rs"]
mod layout;
#[path = "workspace_ui/page.rs"]
mod page;
#[cfg(test)]
#[path = "workspace_ui/tests.rs"]
mod tests;
#[path = "workspace_ui/theme.rs"]
mod theme;

pub use background::BackgroundCarouselState;
pub use command_palette::CommandPaletteState;
pub use language::LanguageMode;
pub use layout::{
    DEFAULT_ACTIVITY_PANEL_WIDTH, DEFAULT_HOSTS_PANEL_WIDTH, DEFAULT_TOOL_PANEL_WIDTH,
    HostListMode, MAX_ACTIVITY_PANEL_WIDTH, MAX_HOSTS_PANEL_WIDTH, MAX_TOOL_PANEL_WIDTH,
    MIN_ACTIVITY_PANEL_WIDTH, MIN_HOSTS_PANEL_WIDTH, MIN_TOOL_PANEL_WIDTH, ToolPanelMode,
};
pub use page::WorkspacePage;
pub use theme::BuiltInTheme;

/// 顶层工作区 UI 状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceUiState {
    pub active_page: WorkspacePage,
    pub host_list_mode: HostListMode,
    pub host_search_query: String,
    pub hosts_panel_width: i32,
    pub activity_panel_width: i32,
    pub tool_panel_width: i32,
    pub tool_panel_mode: ToolPanelMode,
    pub right_sidebar_collapsed: bool,
    pub command_palette: CommandPaletteState,
    pub language: LanguageMode,
    pub theme: BuiltInTheme,
    pub background_carousel: BackgroundCarouselState,
}

impl Default for WorkspaceUiState {
    fn default() -> Self {
        Self {
            active_page: WorkspacePage::Hosts,
            host_list_mode: HostListMode::List,
            host_search_query: String::new(),
            hosts_panel_width: DEFAULT_HOSTS_PANEL_WIDTH,
            activity_panel_width: DEFAULT_ACTIVITY_PANEL_WIDTH,
            tool_panel_width: DEFAULT_TOOL_PANEL_WIDTH,
            tool_panel_mode: ToolPanelMode::Closed,
            right_sidebar_collapsed: false,
            command_palette: CommandPaletteState::default(),
            language: LanguageMode::FollowSystem,
            theme: BuiltInTheme::ProfessionalDark,
            background_carousel: BackgroundCarouselState::default(),
        }
    }
}
