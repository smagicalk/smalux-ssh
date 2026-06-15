//! 工作区布局与分栏状态操作。

use serde::{Deserialize, Serialize};

use crate::model::GroupId;

use super::WorkspaceUiState;

pub const DEFAULT_HOSTS_PANEL_WIDTH: i32 = 276;
pub const MIN_HOSTS_PANEL_WIDTH: i32 = 196;
pub const MAX_HOSTS_PANEL_WIDTH: i32 = 460;
pub const DEFAULT_ACTIVITY_PANEL_WIDTH: i32 = 244;
pub const MIN_ACTIVITY_PANEL_WIDTH: i32 = 184;
pub const MAX_ACTIVITY_PANEL_WIDTH: i32 = 420;
pub const DEFAULT_TOOL_PANEL_WIDTH: i32 = 328;
pub const MIN_TOOL_PANEL_WIDTH: i32 = 220;
pub const MAX_TOOL_PANEL_WIDTH: i32 = 560;

/// Hosts 列表展示方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostListMode {
    #[serde(alias = "List")]
    Tree,
    Card,
}

impl Default for HostListMode {
    fn default() -> Self {
        Self::Tree
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
    KnownHosts,
}

impl Default for ToolPanelMode {
    fn default() -> Self {
        Self::Closed
    }
}

impl WorkspaceUiState {
    /// 切换右侧栏折叠状态。
    pub fn toggle_right_sidebar(&mut self) {
        self.right_sidebar_collapsed = !self.right_sidebar_collapsed;
    }

    /// 切换 Host 列表展示模式。
    pub fn toggle_host_list_mode(&mut self) {
        self.host_list_mode = match self.host_list_mode {
            HostListMode::Tree => HostListMode::Card,
            HostListMode::Card => HostListMode::Tree,
        };
    }

    /// 更新 Hosts 面板搜索条件。
    pub fn set_host_search_query(&mut self, query: impl Into<String>) {
        self.host_search_query = query.into();
    }

    /// 更新密钥页分组树搜索条件。
    pub fn set_credential_search_query(&mut self, query: impl Into<String>) {
        self.credential_search_query = query.into();
    }

    /// 更新片段页分组树搜索条件。
    pub fn set_snippet_search_query(&mut self, query: impl Into<String>) {
        self.snippet_search_query = query.into();
    }

    /// 更新网络资源页搜索条件。
    pub fn set_network_search_query(&mut self, query: impl Into<String>) {
        self.network_search_query = query.into();
    }

    /// 折叠或展开主机树中的指定分组。
    pub fn toggle_host_tree_group(&mut self, group_id: Option<GroupId>) {
        let Some(group_id) = group_id else {
            self.host_tree_root_collapsed = !self.host_tree_root_collapsed;
            return;
        };

        if let Some(index) = self
            .collapsed_host_tree_groups
            .iter()
            .position(|id| *id == group_id)
        {
            self.collapsed_host_tree_groups.remove(index);
        } else {
            self.collapsed_host_tree_groups.push(group_id);
        }
    }

    /// 折叠或展开凭据树中的指定节点。
    pub fn toggle_credential_tree_node(&mut self, node_id: impl Into<String>) {
        let node_id = node_id.into();
        if node_id.is_empty() {
            return;
        }

        if let Some(index) = self
            .collapsed_credential_tree_nodes
            .iter()
            .position(|id| *id == node_id)
        {
            self.collapsed_credential_tree_nodes.remove(index);
        } else {
            self.collapsed_credential_tree_nodes.push(node_id);
        }
    }

    /// 折叠或展开片段树中的指定节点。
    pub fn toggle_snippet_tree_node(&mut self, node_id: impl Into<String>) {
        let node_id = node_id.into();
        if node_id.is_empty() {
            return;
        }

        if let Some(index) = self
            .collapsed_snippet_tree_nodes
            .iter()
            .position(|id| *id == node_id)
        {
            self.collapsed_snippet_tree_nodes.remove(index);
        } else {
            self.collapsed_snippet_tree_nodes.push(node_id);
        }
    }

    /// 更新新建会话弹窗搜索条件。
    pub fn set_new_session_search_query(&mut self, query: impl Into<String>) {
        self.new_session_search_query = query.into();
    }

    /// 更新 Hosts 面板宽度，限制在可用区间内。
    pub fn set_hosts_panel_width(&mut self, width: i32) {
        self.hosts_panel_width = width.clamp(MIN_HOSTS_PANEL_WIDTH, MAX_HOSTS_PANEL_WIDTH);
        self.hosts_panel_collapsed = false;
    }

    /// 折叠或展开 Hosts 面板。
    pub fn set_hosts_panel_collapsed(&mut self, collapsed: bool) {
        self.hosts_panel_collapsed = collapsed;
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
