//! 核心状态到 UI 展示模型的转换。
//!
//! 这里是核心领域层和 Slint 表现层之间的边界，避免 Slint 投影直接散落业务字段读取。

mod activity;
mod common;
mod hosts;
mod labels;
mod palette;
mod sftp;
mod tabs;

use crate::model::AppState;

pub(super) use activity::ActivityViewModel;
use activity::activity;
use common::background_summary;
pub(super) use hosts::HostViewModel;
use hosts::hosts;
use labels::{host_list_mode_label, page_label, theme_label};
pub(super) use palette::CommandPaletteItemViewModel;
use palette::command_palette_results;
use sftp::active_sftp;
pub(super) use sftp::{SftpEntryViewModel, SftpViewModel};
pub(super) use tabs::active_terminal;
use tabs::tabs;
pub(super) use tabs::{SessionTabViewModel, TerminalViewModel};

/// 右侧工具分栏的通用列表项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToolItemViewModel {
    pub title: String,
    pub subtitle: String,
    pub meta: String,
}

/// Known Hosts 工具分栏的展示项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KnownHostViewModel {
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
    pub status: String,
}

/// Slint 窗口所需的完整展示模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppViewModel {
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
pub(super) fn app_view_model(state: &AppState) -> AppViewModel {
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

fn snippet_items(state: &AppState) -> Vec<ToolItemViewModel> {
    state
        .storage
        .snippets
        .iter()
        .map(|snippet| ToolItemViewModel {
            title: snippet.name.clone(),
            subtitle: snippet
                .description
                .clone()
                .unwrap_or_else(|| snippet.command_template.clone()),
            meta: format!("{} vars", snippet.variables.len()),
        })
        .collect()
}

fn tunnel_items(state: &AppState) -> Vec<ToolItemViewModel> {
    let saved = state
        .storage
        .tunnel_rules
        .iter()
        .map(|rule| ToolItemViewModel {
            title: rule.name.clone(),
            subtitle: rule.display_endpoint(),
            meta: if rule.auto_start { "auto" } else { "saved" }.to_owned(),
        });
    let runtime = state
        .sessions
        .tunnels
        .iter()
        .map(|tunnel| ToolItemViewModel {
            title: tunnel.rule_name.clone(),
            subtitle: tunnel
                .last_error
                .clone()
                .unwrap_or_else(|| "runtime".to_owned()),
            meta: format!("{:?}", tunnel.status),
        });

    saved.chain(runtime).collect()
}

fn known_host_items(state: &AppState) -> Vec<KnownHostViewModel> {
    state
        .storage
        .known_hosts
        .iter()
        .map(|entry| KnownHostViewModel {
            host: entry.host.clone(),
            port: entry.port,
            fingerprint: entry.fingerprint.clone(),
            status: if entry.trusted { "trusted" } else { "pending" }.to_owned(),
        })
        .collect()
}

fn tool_panel_mode_label(mode: crate::model::ToolPanelMode) -> &'static str {
    match mode {
        crate::model::ToolPanelMode::Closed => "Closed",
        crate::model::ToolPanelMode::Sftp => "SFTP",
        crate::model::ToolPanelMode::Snippets => "Snippets",
        crate::model::ToolPanelMode::History => "History",
        crate::model::ToolPanelMode::Tunnels => "Tunnels",
        crate::model::ToolPanelMode::KnownHosts => "KnownHosts",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthProfile, Host, HostId, KeyAlgorithm, KnownHostEntry, SecretRef};
    use uuid::Uuid;

    #[test]
    fn app_view_model_uses_local_terminal_when_no_tab_is_open() {
        let state = AppState::default();

        let vm = app_view_model(&state);

        assert_eq!(
            vm.terminal.title,
            crate::model::DEFAULT_LOCAL_TERMINAL_TITLE
        );
        assert_eq!(vm.terminal.status, "Ready");
        assert!(vm.terminal.can_send_input);
    }

    #[test]
    fn auth_label_covers_password_without_secret_leakage() {
        let mut state = AppState::default();
        state.storage.upsert_host(Host {
            id: HostId(Uuid::new_v4()),
            name: "root".to_owned(),
            group_id: None,
            tags: Vec::new(),
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Password {
                username: "root".to_owned(),
                secret: SecretRef("password:root".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        });

        let vm = app_view_model(&state);

        assert_eq!(vm.hosts[0].auth, "Password");
    }

    #[test]
    fn app_view_model_projects_known_hosts_for_tool_panel() {
        let mut state = AppState::default();
        state.storage.upsert_known_host(KnownHostEntry::untrusted(
            "example.com",
            22,
            KeyAlgorithm::Ed25519,
            "SHA256:new",
        ));

        let vm = app_view_model(&state);

        assert_eq!(vm.known_hosts.len(), 1);
        assert_eq!(vm.known_hosts[0].host, "example.com");
        assert_eq!(vm.known_hosts[0].port, 22);
        assert_eq!(vm.known_hosts[0].fingerprint, "SHA256:new");
        assert_eq!(vm.known_hosts[0].status, "pending");
    }
}
