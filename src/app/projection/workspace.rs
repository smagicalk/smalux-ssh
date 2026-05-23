//! Slint 工作区状态写入。

use crate::app::AppWindow;
use crate::app::view_model::AppViewModel;

pub(super) fn sync_workspace_state(window: &AppWindow, model: &AppViewModel) {
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
