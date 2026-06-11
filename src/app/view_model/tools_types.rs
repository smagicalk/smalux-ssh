//! 工具页展示模型类型。

/// 右侧工具分栏的通用列表项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct ToolItemViewModel {
    pub title: String,
    pub subtitle: String,
    pub meta: String,
}

/// Network 页左侧导航和右侧详情共用的行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct NetworkNavItemViewModel {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub meta: String,
    pub kind_key: &'static str,
    pub kind_label: String,
    pub note: String,
    pub icon_key: &'static str,
    pub accent_index: i32,
    pub session_id: String,
    pub primary_action_key: &'static str,
    pub primary_action_label: String,
    pub primary_action_enabled: bool,
    pub stat_primary_label: String,
    pub stat_primary_value: String,
    pub stat_secondary_label: String,
    pub stat_secondary_value: String,
    pub detail_primary_label: String,
    pub detail_primary_value: String,
    pub detail_secondary_label: String,
    pub detail_secondary_value: String,
    pub body_label: String,
    pub body_value: String,
    pub asset_id: String,
    pub edit_kind_key: String,
    pub edit_host: String,
    pub edit_port: String,
    pub edit_tags: String,
    pub edit_bind_host: String,
    pub edit_bind_port: String,
    pub edit_target_host: String,
    pub edit_target_port: String,
    pub edit_auto_start: bool,
    pub edit_host_ids: String,
}

/// 片段页左侧虚拟文件夹树和右侧详情共用的行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SnippetRowViewModel {
    pub id: String,
    pub parent_id: String,
    pub name: String,
    pub description: String,
    pub command_template: String,
    pub scope: String,
    pub scope_key: &'static str,
    pub variables: String,
    pub variable_names: String,
    pub arguments: String,
    pub argument_values: String,
    pub meta: String,
    pub target_linux_selected: bool,
    pub target_debian_selected: bool,
    pub target_rhel_selected: bool,
    pub target_alpine_selected: bool,
    pub target_fedora_selected: bool,
    pub target_arch_selected: bool,
    pub target_suse_selected: bool,
    pub target_freebsd_selected: bool,
    pub target_macos_selected: bool,
    pub target_powershell_selected: bool,
    pub target_cmd_selected: bool,
    pub target_linux_disabled: bool,
    pub target_debian_disabled: bool,
    pub target_rhel_disabled: bool,
    pub target_alpine_disabled: bool,
    pub target_fedora_disabled: bool,
    pub target_arch_disabled: bool,
    pub target_suse_disabled: bool,
    pub target_freebsd_disabled: bool,
    pub target_macos_disabled: bool,
    pub target_powershell_disabled: bool,
    pub target_cmd_disabled: bool,
    pub icon_key: &'static str,
    pub depth: i32,
    pub node_kind: &'static str,
    pub accent_index: i32,
    pub expandable: bool,
    pub expanded: bool,
    pub has_next_sibling: bool,
    pub guide_0: bool,
    pub guide_1: bool,
    pub guide_2: bool,
    pub guide_3: bool,
    pub guide_4: bool,
    pub guide_5: bool,
    pub guide_6: bool,
    pub guide_7: bool,
}

/// 密钥页左侧树和右侧详情共用的凭据行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct CredentialRowViewModel {
    pub id: String,
    pub name: String,
    pub group_id: String,
    pub group_path: String,
    pub kind_key: &'static str,
    pub kind: String,
    pub username: String,
    pub secret_ref: String,
    pub secret_available: bool,
    pub algorithm: String,
    pub algorithm_key: String,
    pub fingerprint: String,
    pub meta: String,
    pub icon_key: &'static str,
    pub depth: i32,
    pub node_kind: &'static str,
    pub accent_index: i32,
    pub expandable: bool,
    pub expanded: bool,
    pub has_next_sibling: bool,
    pub guide_0: bool,
    pub guide_1: bool,
    pub guide_2: bool,
    pub guide_3: bool,
    pub guide_4: bool,
    pub guide_5: bool,
    pub guide_6: bool,
    pub guide_7: bool,
}

/// 凭据分组选中后，右侧详情区展示的直接子项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct CredentialGroupContentViewModel {
    pub parent_id: String,
    pub id: String,
    pub name: String,
    pub group_id: String,
    pub group_path: String,
    pub kind_key: &'static str,
    pub kind: String,
    pub node_kind: &'static str,
    pub username: String,
    pub secret_ref: String,
    pub secret_available: bool,
    pub algorithm: String,
    pub algorithm_key: String,
    pub fingerprint: String,
    pub detail: String,
    pub meta: String,
    pub icon_key: &'static str,
    pub accent_index: i32,
}

/// 凭据详情页的可复制字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct CredentialDetailFieldViewModel {
    pub credential_id: String,
    pub label: String,
    pub value: String,
    pub row: i32,
    pub col: i32,
}

/// Known Hosts 工具分栏的展示项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct KnownHostViewModel {
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
    pub status_key: String,
    pub status: String,
}
