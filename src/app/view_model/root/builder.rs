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
use super::super::palette::command_palette_results;
use super::super::settings::settings;
use super::super::sftp::active_sftp;
use super::super::tabs::{active_terminal, tabs};
use super::super::tools::{credential_items, known_host_items, snippet_items, tunnel_items};
use super::types::{AppViewModel, WorkspaceText};

/// 从核心状态创建 UI 展示模型。
pub(in crate::app) fn app_view_model(state: &AppState) -> AppViewModel {
    let locale = locale_for_state(state);

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
        settings: settings(state),
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
        tool_panel_width: state.ui.workspace.tool_panel_width,
        tool_panel_mode_key: tool_panel_mode_key(state.ui.workspace.tool_panel_mode),
        tool_panel_mode: tool_panel_mode_label(state.ui.workspace.tool_panel_mode, locale),
        right_sidebar_collapsed: state.ui.workspace.right_sidebar_collapsed,
        terminal: active_terminal(state),
        sftp: active_sftp(state),
        last_error: state.ui.last_error.clone().unwrap_or_default(),
        command_palette_open: state.ui.workspace.command_palette.open,
        command_palette_query: state.ui.workspace.command_palette.query.clone(),
        hosts: hosts(state),
        host_tree: host_tree(state),
        new_session_search_query: state.ui.workspace.new_session_search_query.clone(),
        new_session_local_visible: new_session_local_visible(state),
        new_session_hosts: new_session_hosts(state),
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
        credentials: credential_items(state),
        known_hosts: known_host_items(state),
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

fn workspace_text(state: &AppState) -> WorkspaceText {
    let locale = locale_for_state(state);

    WorkspaceText {
        brand_short: tr(locale, "brand.short"),
        brand_name: tr(locale, "brand.name"),
        topbar_find: tr(locale, "topbar.find"),
        topbar_settings: tr(locale, "topbar.settings"),
        nav_hosts: tr(locale, "nav.hosts"),
        nav_terminal: tr(locale, "nav.terminal"),
        nav_sftp: tr(locale, "nav.sftp"),
        nav_tunnels: tr(locale, "nav.tunnels"),
        nav_snippets: tr(locale, "nav.snippets"),
        nav_history: tr(locale, "nav.history"),
        nav_security: tr(locale, "nav.security"),
        nav_settings: tr(locale, "nav.settings"),
        host_new: tr(locale, "host.new"),
        host_open: tr(locale, "host.open"),
        host_files: tr(locale, "host.files"),
        host_edit: tr(locale, "host.edit"),
        host_delete: tr(locale, "host.delete"),
        host_duplicate: tr(locale, "host.duplicate"),
        host_delete_title: tr(locale, "host.delete_title"),
        host_delete_caption: tr(locale, "host.delete_caption"),
        host_delete_confirm: tr(locale, "host.delete_confirm"),
        host_delete_cancel: tr(locale, "host.delete_cancel"),
        group_create_host: tr(locale, "group.create_host"),
        group_create_group: tr(locale, "group.create_group"),
        group_delete_title: tr(locale, "group.delete_title"),
        group_delete_empty_caption: tr(locale, "group.delete_empty_caption"),
        group_delete_non_empty_caption: tr(locale, "group.delete_non_empty_caption"),
        group_delete_confirm: tr(locale, "group.delete_confirm"),
        group_delete_cancel: tr(locale, "group.delete_cancel"),
        tool_files: tr(locale, "tool.files"),
        tool_snippets: tr(locale, "tool.snippets"),
        tool_history: tr(locale, "tool.history"),
        tool_tunnels: tr(locale, "tool.tunnels"),
        tool_keys: tr(locale, "tool.keys"),
        host_search_placeholder: tr(locale, "host.search_placeholder"),
        hosts_section_title: tr(locale, "host.section_title"),
        hosts_empty_title: tr(locale, "host.empty_title"),
        hosts_empty_caption: tr(locale, "host.empty_caption"),
        activity_title: tr(locale, "activity.title"),
        recent_title: tr(locale, "activity.recent_title"),
        recent_empty: tr(locale, "activity.recent_empty"),
        command_palette_title: tr(locale, "palette.title"),
        command_palette_placeholder: tr(locale, "palette.placeholder"),
        command_palette_empty: tr(locale, "palette.empty"),
        command_palette_close: tr(locale, "palette.close"),
        new_session_title: tr(locale, "new_session.title"),
        new_session_search_placeholder: tr(locale, "new_session.search_placeholder"),
        new_session_local_title: tr(locale, "new_session.local_title"),
        new_session_local_subtitle: tr(locale, "new_session.local_subtitle"),
        new_session_local_detail: tr(locale, "new_session.local_detail"),
        new_session_local_tags: tr(locale, "new_session.local_tags"),
        new_session_local_status: tr(locale, "new_session.local_status"),
        new_session_local_kind: tr(locale, "new_session.local_kind"),
        new_session_remote_kind: tr(locale, "new_session.remote_kind"),
        new_session_ungrouped_detail: tr(locale, "new_session.ungrouped_detail"),
        new_session_empty_title: tr(locale, "new_session.empty_title"),
        new_session_empty_caption: tr(locale, "new_session.empty_caption"),
        terminal_reconnect: tr(locale, "terminal.reconnect"),
        tab_ready: tr(locale, "tab.ready"),
        sftp_loading: tr(locale, "sftp.loading"),
        sftp_split_title: tr(locale, "sftp.split_title"),
        sftp_refresh: tr(locale, "sftp.refresh"),
        sftp_local_title: tr(locale, "sftp.local_title"),
        sftp_remote_title: tr(locale, "sftp.remote_title"),
        sftp_workspace_label: tr(locale, "sftp.workspace"),
        sftp_queue_label: tr(locale, "sftp.queue"),
        sftp_selected_label: tr(locale, "sftp.selected"),
        sftp_path_label: tr(locale, "sftp.path"),
        sftp_entries_label: tr(locale, "sftp.entries"),
        sftp_loading_entries: tr(locale, "sftp.loading_entries"),
        sftp_empty_entries: tr(locale, "sftp.empty_entries"),
        tool_pinned: tr(locale, "tool.pinned"),
        tool_empty_value: tr(locale, "tool.empty_value"),
        snippets_empty: tr(locale, "tool.snippets_empty"),
        history_commands: tr(locale, "tool.history_commands"),
        history_saved_suffix: tr(locale, "tool.history_saved_suffix"),
        history_empty: tr(locale, "tool.history_empty"),
        history_run: tr(locale, "tool.history_run"),
        tunnels_empty: tr(locale, "tool.tunnels_empty"),
        known_hosts_empty: tr(locale, "tool.known_hosts_empty"),
        known_hosts_trust: tr(locale, "tool.known_hosts_trust"),
        known_hosts_delete: tr(locale, "tool.known_hosts_delete"),
    }
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
