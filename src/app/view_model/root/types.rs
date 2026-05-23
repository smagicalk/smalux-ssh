//! App 根展示模型类型。

use super::super::activity::ActivityViewModel;
use super::super::hosts::HostViewModel;
use super::super::palette::CommandPaletteItemViewModel;
use super::super::sftp::SftpViewModel;
use super::super::tabs::{SessionTabViewModel, TerminalViewModel};
use super::super::tools::{KnownHostViewModel, ToolItemViewModel};

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
