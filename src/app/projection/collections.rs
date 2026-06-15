//! Slint 列表模型写入。
//!
//! Slint 的列表属性需要 `ModelRc`，而核心 view model 只是普通 `Vec`。这个模块负责集中
//! 完成转换，避免每个 projection 文件都手写 `VecModel` 构造。

use crate::app::AppWindow;
use crate::app::projection::models::{
    activity_model, command_palette_model, credential_detail_field_model,
    credential_group_content_model, credential_option_model, credential_row_model,
    group_option_model, host_model, host_tree_model, known_host_model, network_item_model,
    network_resource_option_model, setting_option_model, settings_file_action_model,
    settings_profile_model, settings_summary_model, sftp_entry_model, snippet_row_model,
    string_model, tab_model, tool_item_model,
};
use crate::app::view_model::AppViewModel;

pub(super) fn sync_collection_models(window: &AppWindow, model: &AppViewModel) {
    let terminal_page = &model.terminal_workspace;
    let security_page = &model.security_workspace;
    let snippet_page = &model.snippet_workspace;
    let settings_page = &model.settings_workspace;

    // 主机首页同时有卡片列表和树列表，它们来自同一个核心状态但投影形状不同。
    window.set_hosts(host_model(&model.hosts));
    window.set_host_tree(host_tree_model(&model.host_tree));
    window.set_new_session_hosts(host_model(&model.new_session_hosts));
    // 创建/编辑弹窗使用分组选项列表，选中状态已经在 view_model 里计算好。
    window.set_quick_host_groups(group_option_model(&model.quick_host.group_options));
    window.set_quick_host_private_key_options(credential_option_model(
        &model.quick_host.private_key_options,
    ));
    window.set_quick_host_certificate_options(credential_option_model(
        &model.quick_host.certificate_options,
    ));
    window.set_quick_host_network_proxy_options(network_resource_option_model(
        &model.quick_host.network_proxy_options,
    ));
    window.set_quick_host_network_jump_options(network_resource_option_model(
        &model.quick_host.network_jump_chain_options,
    ));
    window.set_quick_host_network_forward_options(network_resource_option_model(
        &model.quick_host.network_forward_options,
    ));
    window.set_quick_group_parent_options(group_option_model(
        &model.create_group_dialog.parent_options,
    ));
    // 终端、SFTP、工具面板等集合都只在这里写入 Slint，不回头修改状态。
    window.set_tabs(tab_model(&terminal_page.tabs));
    window.set_sftp_entries(sftp_entry_model(&terminal_page.sftp.entries));
    window.set_activity(activity_model(&model.activity));
    window.set_command_palette_results(command_palette_model(&model.command_palette_results));
    window.set_recent(string_model(&model.recent));
    window.set_history(string_model(&terminal_page.history));
    window.set_snippets(tool_item_model(&snippet_page.snippets));
    window.set_snippet_rows(snippet_row_model(&snippet_page.rows));
    window.set_snippet_target_options(snippet_row_model(&snippet_page.target_options));
    window.set_tunnels(tool_item_model(&terminal_page.tunnels));
    window
        .set_network_runtime_tunnels(network_item_model(&model.network_workspace.runtime_tunnels));
    window.set_network_jump_chain_assets(network_item_model(
        &model.network_workspace.jump_chain_assets,
    ));
    window.set_network_proxy_assets(network_item_model(&model.network_workspace.proxy_assets));
    window.set_network_forward_assets(network_item_model(&model.network_workspace.forward_assets));
    window.set_credentials(tool_item_model(&security_page.credentials));
    window.set_credential_rows(credential_row_model(&security_page.credential_rows));
    window.set_credential_group_contents(credential_group_content_model(
        &security_page.group_contents,
    ));
    window
        .set_credential_detail_fields(credential_detail_field_model(&security_page.detail_fields));
    window.set_known_hosts(known_host_model(&terminal_page.known_hosts));
    // 设置页拆成选项、主题 profile、存储摘要和文件操作，便于 UI 自由布局。
    window.set_settings_language_options(setting_option_model(
        &settings_page.settings.language_options,
    ));
    window.set_settings_theme_options(setting_option_model(&settings_page.settings.theme_options));
    window.set_settings_theme_profiles(settings_profile_model(
        &settings_page.settings.theme.custom_theme_profiles,
    ));
    window.set_settings_storage_summary(settings_summary_model(
        &settings_page.settings.storage.summary_items,
    ));
    window.set_settings_file_actions(settings_file_action_model(
        &settings_page.settings.file_actions,
    ));
}
