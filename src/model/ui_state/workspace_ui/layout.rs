//! 工作区布局与分栏状态操作。

use super::{
    HostListMode, MAX_ACTIVITY_PANEL_WIDTH, MAX_HOSTS_PANEL_WIDTH, MAX_TOOL_PANEL_WIDTH,
    MIN_ACTIVITY_PANEL_WIDTH, MIN_HOSTS_PANEL_WIDTH, MIN_TOOL_PANEL_WIDTH, ToolPanelMode,
    WorkspaceUiState,
};

impl WorkspaceUiState {
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
}
