//! Slint 列表模型写入。
//!
//! Slint 的列表属性需要 `ModelRc`，而核心 view model 只是普通 `Vec`。这个模块负责集中
//! 完成转换，避免每个 projection 文件都手写 `VecModel` 构造。

use crate::app::AppWindow;
use crate::app::projection::models::{
    activity_model, command_palette_model, group_option_model, host_model, host_tree_model,
    known_host_model, setting_option_model, settings_file_action_model, settings_profile_model,
    settings_summary_model, sftp_entry_model, string_model, tab_model, tool_item_model,
};
use crate::app::view_model::AppViewModel;

pub(super) fn sync_collection_models(window: &AppWindow, model: &AppViewModel) {
    // 主机首页同时有卡片列表和树列表，它们来自同一个核心状态但投影形状不同。
    window.set_hosts(host_model(&model.hosts));
    window.set_host_tree(host_tree_model(&model.host_tree));
    window.set_new_session_hosts(host_model(&model.new_session_hosts));
    // 创建/编辑弹窗使用分组选项列表，选中状态已经在 view_model 里计算好。
    window.set_quick_host_groups(group_option_model(&model.quick_host.group_options));
    window.set_quick_group_parent_options(group_option_model(
        &model.create_group_dialog.parent_options,
    ));
    // 终端、SFTP、工具面板等集合都只在这里写入 Slint，不回头修改 AppState。
    window.set_tabs(tab_model(&model.tabs));
    window.set_sftp_entries(sftp_entry_model(&model.sftp.entries));
    window.set_activity(activity_model(&model.activity));
    window.set_command_palette_results(command_palette_model(&model.command_palette_results));
    window.set_recent(string_model(&model.recent));
    window.set_history(string_model(&model.history));
    window.set_snippets(tool_item_model(&model.snippets));
    window.set_tunnels(tool_item_model(&model.tunnels));
    window.set_credentials(tool_item_model(&model.credentials));
    window.set_known_hosts(known_host_model(&model.known_hosts));
    // 设置页拆成选项、主题 profile、存储摘要和文件操作，便于 UI 自由布局。
    window.set_settings_language_options(setting_option_model(&model.settings.language_options));
    window.set_settings_theme_options(setting_option_model(&model.settings.theme_options));
    window.set_settings_theme_profiles(settings_profile_model(
        &model.settings.theme.custom_theme_profiles,
    ));
    window.set_settings_storage_summary(settings_summary_model(
        &model.settings.storage.summary_items,
    ));
    window.set_settings_file_actions(settings_file_action_model(&model.settings.file_actions));
}
