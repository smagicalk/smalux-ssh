//! Slint 属性写入层。
//!
//! 本模块只把 UI 展示模型写入 Slint 窗口，不直接读取核心领域细节。

use slint::{ModelRc, SharedString, VecModel};

use crate::model::AppState;

use super::view_model::{
    ActivityViewModel, AppViewModel, CommandPaletteItemViewModel, HostViewModel,
    KnownHostViewModel, SessionTabViewModel, SftpEntryViewModel, ToolItemViewModel,
    active_terminal, app_view_model,
};
use super::{
    ActivityRow, AppWindow, CommandPaletteRow, HostRow, KnownHostRow, SessionTabRow, SftpEntryRow,
    ToolItemRow,
};

/// 将当前应用状态同步到 Slint 窗口。
pub(super) fn sync_window(window: &AppWindow, state: &AppState) {
    sync_view_model(window, &app_view_model(state));
}

/// 只同步当前终端面板，用于回车和本地 PTY 输出刷新。
pub(super) fn sync_terminal_pane(window: &AppWindow, state: &AppState) {
    let model = active_terminal(state);
    window.set_active_session_id(model.session_id.as_str().into());
    window.set_active_tab_title(model.title.as_str().into());
    window.set_active_tab_kind(model.kind.into());
    window.set_active_tab_status(model.status.into());
    window.set_terminal_input(model.input.as_str().into());
    window.set_terminal_prompt(model.prompt.into());
    window.set_terminal_output(string_model(&model.output_lines));
    window.set_terminal_can_send_input(model.can_send_input);
}

fn sync_view_model(window: &AppWindow, model: &AppViewModel) {
    window.set_host_count(model.host_count.as_str().into());
    window.set_tab_count(model.tab_count.as_str().into());
    window.set_active_page(model.active_page.into());
    window.set_language_label(model.language_label.into());
    window.set_theme_name(model.theme_name.into());
    window.set_background_summary(model.background_summary.as_str().into());
    window.set_host_list_mode(model.host_list_mode.into());
    window.set_host_search_query(model.host_search_query.as_str().into());
    window.set_hosts_panel_width(model.hosts_panel_width);
    window.set_activity_panel_width(model.activity_panel_width);
    window.set_tool_panel_width(model.tool_panel_width);
    window.set_tool_panel_mode(model.tool_panel_mode.into());
    window.set_right_sidebar_collapsed(model.right_sidebar_collapsed);
    window.set_last_error(model.last_error.as_str().into());
    window.set_command_palette_open(model.command_palette_open);
    window.set_command_palette_query(model.command_palette_query.as_str().into());
    window.set_active_session_id(model.terminal.session_id.as_str().into());
    window.set_active_tab_title(model.terminal.title.as_str().into());
    window.set_active_tab_kind(model.terminal.kind.into());
    window.set_active_tab_status(model.terminal.status.into());
    window.set_terminal_input(model.terminal.input.as_str().into());
    window.set_terminal_prompt(model.terminal.prompt.into());
    window.set_terminal_output(string_model(&model.terminal.output_lines));
    window.set_terminal_can_send_input(model.terminal.can_send_input);
    window.set_sftp_host_id(model.sftp.host_id.as_str().into());
    window.set_sftp_title(model.sftp.title.as_str().into());
    window.set_sftp_current_dir(model.sftp.current_dir.as_str().into());
    window.set_sftp_selected_path(model.sftp.selected_path.as_str().into());
    window.set_sftp_loading(model.sftp.loading);
    window.set_sftp_error(model.sftp.last_error.as_str().into());
    window.set_hosts(host_model(&model.hosts));
    window.set_tabs(tab_model(&model.tabs));
    window.set_sftp_entries(sftp_entry_model(&model.sftp.entries));
    window.set_activity(activity_model(&model.activity));
    window.set_command_palette_results(command_palette_model(&model.command_palette_results));
    window.set_recent(string_model(&model.recent));
    window.set_history(string_model(&model.history));
    window.set_snippets(tool_item_model(&model.snippets));
    window.set_tunnels(tool_item_model(&model.tunnels));
    window.set_known_hosts(known_host_model(&model.known_hosts));
}

fn host_model(items: &[HostViewModel]) -> ModelRc<HostRow> {
    let rows = items
        .iter()
        .map(|host| HostRow {
            id: host.id.as_str().into(),
            name: host.name.as_str().into(),
            endpoint: host.endpoint.as_str().into(),
            auth: host.auth.into(),
            group: host.group.as_str().into(),
            tags: host.tags.as_str().into(),
            status: host.status.into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn tab_model(items: &[SessionTabViewModel]) -> ModelRc<SessionTabRow> {
    let rows = items
        .iter()
        .map(|tab| SessionTabRow {
            id: tab.id.as_str().into(),
            title: tab.title.as_str().into(),
            kind: tab.kind.into(),
            status: tab.status.into(),
            active: tab.active,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn activity_model(items: &[ActivityViewModel]) -> ModelRc<ActivityRow> {
    let rows = items
        .iter()
        .map(|item| ActivityRow {
            label: item.label.into(),
            value: item.value.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn command_palette_model(items: &[CommandPaletteItemViewModel]) -> ModelRc<CommandPaletteRow> {
    let rows = items
        .iter()
        .map(|item| CommandPaletteRow {
            id: item.id.as_str().into(),
            title: item.title.as_str().into(),
            subtitle: item.subtitle.as_str().into(),
            kind: item.kind.into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn sftp_entry_model(items: &[SftpEntryViewModel]) -> ModelRc<SftpEntryRow> {
    let rows = items
        .iter()
        .map(|item| SftpEntryRow {
            name: item.name.as_str().into(),
            path: item.path.as_str().into(),
            kind: item.kind.into(),
            size: item.size.as_str().into(),
            selected: item.selected,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn tool_item_model(items: &[ToolItemViewModel]) -> ModelRc<ToolItemRow> {
    let rows = items
        .iter()
        .map(|item| ToolItemRow {
            title: item.title.as_str().into(),
            subtitle: item.subtitle.as_str().into(),
            meta: item.meta.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn known_host_model(items: &[KnownHostViewModel]) -> ModelRc<KnownHostRow> {
    let rows = items
        .iter()
        .map(|item| KnownHostRow {
            host: item.host.as_str().into(),
            port: i32::from(item.port),
            fingerprint: item.fingerprint.as_str().into(),
            status: item.status.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn string_model(items: &[String]) -> ModelRc<SharedString> {
    let rows = items
        .iter()
        .map(|item| SharedString::from(item.as_str()))
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
