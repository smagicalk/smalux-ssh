//! App 根展示模型聚合。

use crate::model::AppState;

use super::activity::{ActivityViewModel, activity};
use super::common::background_summary;
use super::hosts::{HostViewModel, hosts};
use super::labels::{host_list_mode_label, page_label, theme_label, tool_panel_mode_label};
use super::palette::{CommandPaletteItemViewModel, command_palette_results};
use super::sftp::{SftpViewModel, active_sftp};
use super::tabs::{SessionTabViewModel, TerminalViewModel, active_terminal, tabs};
use super::tools::{
    KnownHostViewModel, ToolItemViewModel, known_host_items, snippet_items, tunnel_items,
};

/// Slint 窗口所需的完整展示模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct AppViewModel {
    pub host_count: String,
    pub tab_count: String,
    pub active_page: &'static str,
    pub language_label: &'static str,
    pub theme_name: &'static str,
    pub background_summary: String,
    pub host_list_mode: &'static str,
    pub host_search_query: String,
    pub hosts_panel_width: i32,
    pub activity_panel_width: i32,
    pub tool_panel_width: i32,
    pub tool_panel_mode: &'static str,
    pub right_sidebar_collapsed: bool,
    pub terminal: TerminalViewModel,
    pub sftp: SftpViewModel,
    pub last_error: String,
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub hosts: Vec<HostViewModel>,
    pub tabs: Vec<SessionTabViewModel>,
    pub activity: Vec<ActivityViewModel>,
    pub command_palette_results: Vec<CommandPaletteItemViewModel>,
    pub recent: Vec<String>,
    pub history: Vec<String>,
    pub snippets: Vec<ToolItemViewModel>,
    pub tunnels: Vec<ToolItemViewModel>,
    pub known_hosts: Vec<KnownHostViewModel>,
}

/// 从核心状态创建 UI 展示模型。
pub(in crate::app) fn app_view_model(state: &AppState) -> AppViewModel {
    AppViewModel {
        host_count: state.storage.host_count().to_string(),
        tab_count: state.sessions.tab_count().to_string(),
        active_page: page_label(state.ui.workspace.active_page),
        language_label: state.ui.workspace.language_label(),
        theme_name: theme_label(state.ui.workspace.theme),
        background_summary: background_summary(state),
        host_list_mode: host_list_mode_label(state.ui.workspace.host_list_mode),
        host_search_query: state.ui.workspace.host_search_query.clone(),
        hosts_panel_width: state.ui.workspace.hosts_panel_width,
        activity_panel_width: state.ui.workspace.activity_panel_width,
        tool_panel_width: state.ui.workspace.tool_panel_width,
        tool_panel_mode: tool_panel_mode_label(state.ui.workspace.tool_panel_mode),
        right_sidebar_collapsed: state.ui.workspace.right_sidebar_collapsed,
        terminal: active_terminal(state),
        sftp: active_sftp(state),
        last_error: state.ui.last_error.clone().unwrap_or_default(),
        command_palette_open: state.ui.workspace.command_palette.open,
        command_palette_query: state.ui.workspace.command_palette.query.clone(),
        hosts: hosts(state),
        tabs: tabs(state),
        activity: activity(state),
        command_palette_results: command_palette_results(state),
        recent: state
            .storage
            .recent_connections
            .iter()
            .map(|recent| recent.label.clone())
            .collect(),
        history: state
            .storage
            .command_history
            .iter()
            .rev()
            .map(|item| item.command.clone())
            .collect(),
        snippets: snippet_items(state),
        tunnels: tunnel_items(state),
        known_hosts: known_host_items(state),
    }
}
