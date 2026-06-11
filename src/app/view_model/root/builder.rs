//! App 根展示模型构建。

use crate::model::AppState;

use super::super::activity::activity;
use super::super::common::{background_summary, theme_palette};
use super::super::hosts::{
    create_choice_text, create_group_dialog, create_host_dialog_text, host_tree, hosts,
    new_session_hosts, quick_host,
};
use super::super::i18n::{locale_for_state, tr};
use super::super::labels::{
    host_list_mode_key, host_list_mode_label, page_key, page_label, theme_label,
    tool_panel_mode_key, tool_panel_mode_label,
};
use super::super::network::{network_resource_items, runtime_tunnel_items};
use super::super::palette::command_palette_results;
use super::super::settings::settings;
use super::super::sftp::active_sftp;
use super::super::tabs::{active_terminal, tabs};
use super::super::tools::{
    credential_detail_fields, credential_group_contents, credential_items, credential_rows,
    known_host_items, snippet_items, snippet_rows, snippet_target_options, tunnel_items,
};
use super::types::{
    AppViewModel, NetworkWorkspaceViewModel, SecurityWorkspaceViewModel,
    SettingsWorkspaceViewModel, SnippetWorkspaceViewModel, TerminalWorkspaceViewModel,
};
use super::workspace_text::workspace_text;

/// 从核心状态创建 UI 展示模型。
pub(in crate::app) fn app_view_model(state: &AppState) -> AppViewModel {
    let locale = locale_for_state(state);
    let settings = settings(state);
    let terminal = active_terminal(state);
    let sftp = active_sftp(state);
    let tabs = tabs(state);
    let history: Vec<String> = state
        .storage
        .command_history
        .iter()
        .rev()
        .map(|item| item.command.clone())
        .collect();
    let snippets = snippet_items(state);
    let snippet_rows = snippet_rows(state);
    let snippet_target_options = snippet_target_options(state);
    let tunnels = tunnel_items(state);
    let credentials = credential_items(state);
    let credential_rows = credential_rows(state);
    let credential_group_contents = credential_group_contents(state);
    let credential_detail_fields = credential_detail_fields(state);
    let known_hosts = known_host_items(state);
    let runtime_tunnels = runtime_tunnel_items(state);
    let network_resources = network_resource_items(state);

    AppViewModel {
        host_count: state.storage.host_count().to_string(),
        tab_count: state.sessions.tab_count().to_string(),
        active_page_key: page_key(state.ui.workspace.active_page),
        active_page: page_label(state.ui.workspace.active_page, locale),
        language_label: state.ui.workspace.language_label(),
        workspace_text: workspace_text(state),
        theme_name: theme_label(state.ui.workspace.theme, locale),
        theme_palette: theme_palette(state.ui.workspace.theme),
        background_summary: background_summary(state),
        host_list_mode_key: host_list_mode_key(state.ui.workspace.host_list_mode),
        host_list_mode: host_list_mode_label(state.ui.workspace.host_list_mode, locale),
        host_search_query: state.ui.workspace.host_search_query.clone(),
        create_host_dialog_open: state.ui.workspace.create_host_dialog_open,
        create_group_parent_dialog_open: state.ui.workspace.create_group_parent_dialog_open,
        create_group_dialog_open: state.ui.workspace.create_group_dialog_open,
        remove_host_dialog_open: state.ui.workspace.pending_delete_host_id.is_some(),
        remove_host_dialog_name: pending_delete_host_name(state),
        remove_group_dialog_open: state.ui.workspace.pending_delete_group_id.is_some(),
        remove_group_dialog_name: pending_delete_group_name(state),
        remove_group_dialog_caption: pending_delete_group_caption(state),
        create_host_dialog: create_host_dialog_text(state),
        create_choice: create_choice_text(state),
        create_group_dialog: create_group_dialog(state),
        quick_host: quick_host(state),
        hosts_panel_collapsed: state.ui.workspace.hosts_panel_collapsed,
        hosts_panel_width: state.ui.workspace.hosts_panel_width,
        activity_panel_width: state.ui.workspace.activity_panel_width,
        right_sidebar_collapsed: state.ui.workspace.right_sidebar_collapsed,
        terminal_workspace: TerminalWorkspaceViewModel {
            terminal: terminal.clone(),
            sftp: sftp.clone(),
            tabs: tabs.clone(),
            history: history.clone(),
            tunnels: tunnels.clone(),
            known_hosts: known_hosts.clone(),
            tool_panel_width: state.ui.workspace.tool_panel_width,
            tool_panel_mode_key: tool_panel_mode_key(state.ui.workspace.tool_panel_mode),
            tool_panel_mode: tool_panel_mode_label(state.ui.workspace.tool_panel_mode, locale),
        },
        security_workspace: SecurityWorkspaceViewModel {
            search_query: state.ui.workspace.credential_search_query.clone(),
            credentials: credentials.clone(),
            credential_rows: credential_rows.clone(),
            group_contents: credential_group_contents.clone(),
            detail_fields: credential_detail_fields.clone(),
        },
        snippet_workspace: SnippetWorkspaceViewModel {
            search_query: state.ui.workspace.snippet_search_query.clone(),
            snippets: snippets.clone(),
            rows: snippet_rows.clone(),
            target_options: snippet_target_options.clone(),
        },
        network_workspace: NetworkWorkspaceViewModel {
            runtime_tunnels,
            resources: network_resources,
        },
        settings_workspace: SettingsWorkspaceViewModel {
            settings: settings.clone(),
        },
        last_error: state.ui.last_error.clone().unwrap_or_default(),
        command_palette_open: state.ui.workspace.command_palette.open,
        command_palette_query: state.ui.workspace.command_palette.query.clone(),
        hosts: hosts(state),
        host_tree: host_tree(state),
        new_session_search_query: state.ui.workspace.new_session_search_query.clone(),
        new_session_local_visible: new_session_local_visible(state),
        new_session_hosts: new_session_hosts(state),
        activity: activity(state),
        command_palette_results: command_palette_results(state),
        recent: state
            .storage
            .recent_connections
            .iter()
            .map(|recent| recent.label.clone())
            .collect(),
    }
}

fn new_session_local_visible(state: &AppState) -> bool {
    let query = state
        .ui
        .workspace
        .new_session_search_query
        .trim()
        .to_lowercase();
    query.is_empty()
        || [
            "local",
            "terminal",
            "shell",
            "本地",
            "终端",
            "当前工作区",
            "workspace",
        ]
        .iter()
        .any(|candidate| candidate.contains(query.as_str()))
}

fn pending_delete_host_name(state: &AppState) -> String {
    state
        .ui
        .workspace
        .pending_delete_host_id
        .and_then(|host_id| {
            state
                .storage
                .hosts
                .iter()
                .find(|host| host.id == host_id)
                .map(|host| host.name.clone())
        })
        .unwrap_or_default()
}

fn pending_delete_group_name(state: &AppState) -> String {
    state
        .ui
        .workspace
        .pending_delete_group_id
        .and_then(|group_id| {
            state
                .storage
                .groups
                .iter()
                .find(|group| group.id == group_id)
                .map(|group| group.name.clone())
        })
        .unwrap_or_default()
}

fn pending_delete_group_caption(state: &AppState) -> &'static str {
    let locale = locale_for_state(state);
    let Some(group_id) = state.ui.workspace.pending_delete_group_id else {
        return tr(locale, "group.delete_empty_caption");
    };

    if group_has_contents(state, group_id) {
        tr(locale, "group.delete_non_empty_caption")
    } else {
        tr(locale, "group.delete_empty_caption")
    }
}

fn group_has_contents(state: &AppState, group_id: crate::model::GroupId) -> bool {
    state
        .storage
        .hosts
        .iter()
        .any(|host| host.group_id == Some(group_id))
        || state
            .storage
            .groups
            .iter()
            .any(|group| group.parent_id == Some(group_id))
}
