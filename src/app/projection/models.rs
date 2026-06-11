//! Slint 列表模型转换。

mod activity;
mod common;
mod hosts;
mod known_hosts;
mod palette;
mod settings;
mod sftp;
mod tabs;
mod tools;
mod tools_common;
mod tools_credentials;
mod tools_snippets;

pub(super) use activity::activity_model;
pub(super) use common::string_model;
pub(super) use hosts::{
    credential_option_model, group_option_model, host_model, host_tree_model,
    network_resource_option_model,
};
pub(super) use known_hosts::known_host_model;
pub(super) use palette::command_palette_model;
pub(super) use settings::{
    setting_option_model, settings_file_action_model, settings_profile_model,
    settings_summary_model,
};
pub(super) use sftp::sftp_entry_model;
pub(super) use tabs::tab_model;
pub(super) use tools::{
    credential_detail_field_model, credential_group_content_model, credential_row_model,
    network_item_model, snippet_row_model, tool_item_model,
};
