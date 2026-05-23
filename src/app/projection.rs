//! Slint 属性写入层。
//!
//! 本模块只把 UI 展示模型写入 Slint 窗口，不直接读取核心领域细节。

use crate::model::AppState;

use super::AppWindow;
use super::view_model::{AppViewModel, TerminalViewModel, active_terminal, app_view_model};

mod models;

use models::{
    activity_model, command_palette_model, host_model, known_host_model, sftp_entry_model,
    string_model, tab_model, tool_item_model,
};

/// 将当前应用状态同步到 Slint 窗口。
pub(super) fn sync_window(window: &AppWindow, state: &AppState) {
    sync_view_model(window, &app_view_model(state));
}

/// 只同步当前终端面板，用于回车和本地 PTY 输出刷新。
pub(super) fn sync_terminal_pane(window: &AppWindow, state: &AppState) {
    let model = active_terminal(state);
    sync_terminal_model(window, &model);
}

fn sync_view_model(window: &AppWindow, model: &AppViewModel) {
    sync_workspace_state(window, model);
    sync_terminal_model(window, &model.terminal);
    sync_sftp_model(window, model);
    sync_collection_models(window, model);
}

fn sync_workspace_state(window: &AppWindow, model: &AppViewModel) {
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
}

fn sync_terminal_model(window: &AppWindow, model: &TerminalViewModel) {
    window.set_active_session_id(model.session_id.as_str().into());
    window.set_active_tab_title(model.title.as_str().into());
    window.set_active_tab_kind(model.kind.into());
    window.set_active_tab_status(model.status.into());
    window.set_terminal_input(model.input.as_str().into());
    window.set_terminal_prompt(model.prompt.into());
    window.set_terminal_output(string_model(&model.output_lines));
    window.set_terminal_can_send_input(model.can_send_input);
}

fn sync_sftp_model(window: &AppWindow, model: &AppViewModel) {
    window.set_sftp_host_id(model.sftp.host_id.as_str().into());
    window.set_sftp_title(model.sftp.title.as_str().into());
    window.set_sftp_current_dir(model.sftp.current_dir.as_str().into());
    window.set_sftp_selected_path(model.sftp.selected_path.as_str().into());
    window.set_sftp_loading(model.sftp.loading);
    window.set_sftp_error(model.sftp.last_error.as_str().into());
}

fn sync_collection_models(window: &AppWindow, model: &AppViewModel) {
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
