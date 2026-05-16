//! 桌面工作区自身的 UI 状态。

use serde::{Deserialize, Serialize};

pub const DEFAULT_HOSTS_PANEL_WIDTH: i32 = 276;
pub const MIN_HOSTS_PANEL_WIDTH: i32 = 196;
pub const MAX_HOSTS_PANEL_WIDTH: i32 = 460;
pub const DEFAULT_ACTIVITY_PANEL_WIDTH: i32 = 244;
pub const MIN_ACTIVITY_PANEL_WIDTH: i32 = 184;
pub const MAX_ACTIVITY_PANEL_WIDTH: i32 = 420;
pub const DEFAULT_TOOL_PANEL_WIDTH: i32 = 328;
pub const MIN_TOOL_PANEL_WIDTH: i32 = 220;
pub const MAX_TOOL_PANEL_WIDTH: i32 = 560;

/// 当前显示的一级页面。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspacePage {
    Hosts,
    Terminal,
    Sftp,
    Tunnels,
    Snippets,
    History,
    Security,
    Settings,
}

impl Default for WorkspacePage {
    fn default() -> Self {
        Self::Hosts
    }
}

/// Hosts 列表展示方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostListMode {
    List,
    Card,
}

impl Default for HostListMode {
    fn default() -> Self {
        Self::List
    }
}

/// D 区域右侧按需打开的辅助分栏。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolPanelMode {
    Closed,
    Sftp,
    Snippets,
    History,
    Tunnels,
}

impl Default for ToolPanelMode {
    fn default() -> Self {
        Self::Closed
    }
}

/// 命令面板查询状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
        }
    }
}

/// UI 语言选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageMode {
    FollowSystem,
    Chinese,
    English,
}

impl Default for LanguageMode {
    fn default() -> Self {
        Self::FollowSystem
    }
}

/// 可选的内置视觉主题。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltInTheme {
    ProfessionalDark,
    OceanDark,
    ForestDark,
}

impl Default for BuiltInTheme {
    fn default() -> Self {
        Self::ProfessionalDark
    }
}

/// 背景轮播运行态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundCarouselState {
    pub enabled: bool,
    pub active_index: usize,
}

impl Default for BackgroundCarouselState {
    fn default() -> Self {
        Self {
            enabled: true,
            active_index: 0,
        }
    }
}

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

impl WorkspaceUiState {
    /// 打开命令面板并设置查询文本。
    pub fn open_command_palette(&mut self, query: impl Into<String>) {
        self.command_palette.open = true;
        self.command_palette.query = query.into();
    }

    /// 关闭命令面板并清空查询。
    pub fn close_command_palette(&mut self) {
        self.command_palette.open = false;
        self.command_palette.query.clear();
    }

    /// 切换右侧栏折叠状态。
    pub fn toggle_right_sidebar(&mut self) {
        self.right_sidebar_collapsed = !self.right_sidebar_collapsed;
    }

    /// 切换 Host 列表展示模式。
    pub fn toggle_host_list_mode(&mut self) {
        self.host_list_mode = match self.host_list_mode {
            HostListMode::List => HostListMode::Card,
            HostListMode::Card => HostListMode::List,
        };
    }

    /// 更新 Hosts 面板搜索条件。
    pub fn set_host_search_query(&mut self, query: impl Into<String>) {
        self.host_search_query = query.into();
    }

    /// 更新 Hosts 面板宽度，限制在可用区间内。
    pub fn set_hosts_panel_width(&mut self, width: i32) {
        self.hosts_panel_width = width.clamp(MIN_HOSTS_PANEL_WIDTH, MAX_HOSTS_PANEL_WIDTH);
    }

    /// 更新右侧活动栏宽度，限制在可用区间内。
    pub fn set_activity_panel_width(&mut self, width: i32) {
        self.activity_panel_width = width.clamp(MIN_ACTIVITY_PANEL_WIDTH, MAX_ACTIVITY_PANEL_WIDTH);
    }

    /// 更新 D 区域内部工具/SFTP 分栏宽度，限制在可用区间内。
    pub fn set_tool_panel_width(&mut self, width: i32) {
        self.tool_panel_width = width.clamp(MIN_TOOL_PANEL_WIDTH, MAX_TOOL_PANEL_WIDTH);
    }

    /// 打开 D 区域内部辅助分栏。
    pub fn open_tool_panel(&mut self, mode: ToolPanelMode) {
        self.tool_panel_mode = mode;
    }

    /// 关闭 D 区域内部辅助分栏。
    pub fn close_tool_panel(&mut self) {
        self.tool_panel_mode = ToolPanelMode::Closed;
    }

    /// 切换到下一张背景。
    pub fn next_background(&mut self, source_count: usize) {
        if source_count == 0 {
            self.background_carousel.active_index = 0;
            return;
        }

        self.background_carousel.active_index =
            (self.background_carousel.active_index + 1) % source_count;
    }

    /// 返回当前背景索引，自动限制到来源数量内。
    pub fn active_background_index(&self, source_count: usize) -> Option<usize> {
        if source_count == 0 || !self.background_carousel.enabled {
            None
        } else {
            Some(self.background_carousel.active_index % source_count)
        }
    }
}

impl LanguageMode {
    /// 用于设置页展示的语言模式标签。
    pub fn label(self) -> &'static str {
        match self {
            Self::FollowSystem => "system",
            Self::Chinese => "zh-CN",
            Self::English => "en-US",
        }
    }
}

impl WorkspaceUiState {
    /// 返回当前语言模式标签。
    pub fn language_label(&self) -> &'static str {
        self.language.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_ui_defaults_to_hosts_follow_system_language() {
        let state = WorkspaceUiState::default();

        assert_eq!(state.active_page, WorkspacePage::Hosts);
        assert_eq!(state.language, LanguageMode::FollowSystem);
        assert_eq!(state.host_list_mode, HostListMode::List);
        assert!(state.host_search_query.is_empty());
        assert_eq!(state.hosts_panel_width, DEFAULT_HOSTS_PANEL_WIDTH);
        assert_eq!(state.activity_panel_width, DEFAULT_ACTIVITY_PANEL_WIDTH);
        assert_eq!(state.tool_panel_width, DEFAULT_TOOL_PANEL_WIDTH);
        assert_eq!(state.tool_panel_mode, ToolPanelMode::Closed);
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
}
