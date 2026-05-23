//! Slint 列表模型写入。

use crate::app::AppWindow;
use crate::app::projection::models::{
    activity_model, command_palette_model, host_model, known_host_model, sftp_entry_model,
    string_model, tab_model, tool_item_model,
};
use crate::app::view_model::AppViewModel;

pub(super) fn sync_collection_models(window: &AppWindow, model: &AppViewModel) {
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
