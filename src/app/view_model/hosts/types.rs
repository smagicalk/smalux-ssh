//! 主机列表展示行类型。

/// 主机列表展示行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct HostViewModel {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub icon_key: String,
    pub auth: &'static str,
    pub group: String,
    pub group_id: String,
    pub group_header: String,
    pub group_header_id: String,
    pub tags: String,
    pub network_summary: String,
    pub status_key: &'static str,
    pub status: &'static str,
    pub accent_index: i32,
}

/// 主机树展示行，独立表达文件夹和主机节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct HostTreeViewModel {
    pub id: String,
    pub parent_id: String,
    pub name: String,
    pub endpoint: String,
    pub icon_key: String,
    pub kind: &'static str,
    pub host_id: String,
    pub group_id: String,
    pub depth: i32,
    pub expanded: bool,
    pub status_key: &'static str,
    pub accent_index: i32,
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

/// 创建入口弹窗文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct CreateChoiceText {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub host_label: &'static str,
    pub host_caption: &'static str,
    pub group_label: &'static str,
    pub group_caption: &'static str,
    pub cancel_label: &'static str,
}

/// 创建分组弹窗文案和草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct CreateGroupDialogViewModel {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub parent_picker_title: &'static str,
    pub parent_picker_subtitle: &'static str,
    pub name_label: &'static str,
    pub name_placeholder: &'static str,
    pub parent_label: &'static str,
    pub cancel_label: &'static str,
    pub save_label: &'static str,
    pub next_label: &'static str,
    pub parent_path: String,
    pub parent_options: Vec<GroupOptionViewModel>,
    pub name: String,
}

/// 创建主机弹窗中的分组选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct GroupOptionViewModel {
    pub id: String,
    pub name: String,
    pub path: String,
    pub depth: i32,
    pub selected: bool,
}

/// 创建主机弹窗中的凭据选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct CredentialOptionViewModel {
    pub value: String,
    pub label: String,
    pub detail: String,
    pub selected: bool,
}

/// 创建主机弹窗中的网络资源选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct NetworkResourceOptionViewModel {
    pub value: String,
    pub label: String,
    pub detail: String,
    pub kind_key: &'static str,
    pub kind_label: String,
    pub icon_key: &'static str,
    pub accent_index: i32,
    pub selected: bool,
}

/// 创建主机弹窗文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct CreateHostDialogText {
    pub editing: bool,
    pub dialog_title: &'static str,
    pub dialog_subtitle: &'static str,
    pub connection_title: &'static str,
    pub connection_caption: &'static str,
    pub authentication_title: &'static str,
    pub authentication_caption: &'static str,
    pub credential_note: &'static str,
    pub cancel_label: &'static str,
    pub create_label: &'static str,
    pub address_label: &'static str,
    pub address_placeholder: &'static str,
    pub port_label: &'static str,
    pub port_placeholder: &'static str,
    pub username_label: &'static str,
    pub username_placeholder: &'static str,
    pub host_alias_label: &'static str,
    pub host_alias_placeholder: &'static str,
    pub tags_label: &'static str,
    pub tags_placeholder: &'static str,
    pub group_label: &'static str,
    pub group_picker_title: &'static str,
    pub auth_agent_label: &'static str,
    pub auth_agent_caption: &'static str,
    pub auth_password_label: &'static str,
    pub auth_password_caption: &'static str,
    pub auth_key_label: &'static str,
    pub auth_key_caption: &'static str,
    pub auth_certificate_label: &'static str,
    pub auth_certificate_caption: &'static str,
    pub agent_source_title: &'static str,
    pub agent_auto_label: &'static str,
    pub agent_auto_caption: &'static str,
    pub agent_openssh_label: &'static str,
    pub agent_openssh_caption: &'static str,
    pub agent_pageant_label: &'static str,
    pub agent_pageant_caption: &'static str,
    pub agent_custom_label: &'static str,
    pub agent_custom_caption: &'static str,
    pub custom_agent_pipe_label: &'static str,
    pub custom_agent_pipe_placeholder: &'static str,
    pub agent_key_hint_label: &'static str,
    pub agent_key_hint_placeholder: &'static str,
    pub password_secret_label: &'static str,
    pub password_secret_placeholder: &'static str,
    pub private_key_label: &'static str,
    pub private_key_placeholder: &'static str,
    pub add_private_key_label: &'static str,
    pub add_private_key_caption: &'static str,
    pub passphrase_label: &'static str,
    pub passphrase_placeholder: &'static str,
    pub certificate_label: &'static str,
    pub certificate_placeholder: &'static str,
    pub add_certificate_label: &'static str,
    pub add_certificate_caption: &'static str,
    pub credential_name_label: &'static str,
    pub credential_name_placeholder: &'static str,
    pub credential_secret_label: &'static str,
    pub credential_secret_placeholder: &'static str,
    pub credential_algorithm_label: &'static str,
    pub credential_save_label: &'static str,
    pub network_title: &'static str,
    pub network_caption: &'static str,
    pub network_proxy_label: &'static str,
    pub network_jump_label: &'static str,
    pub network_forward_label: &'static str,
    pub network_empty_label: &'static str,
    pub icon_title: &'static str,
    pub icon_server_label: &'static str,
    pub icon_database_label: &'static str,
    pub icon_cloud_label: &'static str,
    pub icon_linux_label: &'static str,
    pub icon_container_label: &'static str,
    pub icon_shield_label: &'static str,
    pub icon_router_label: &'static str,
    pub icon_terminal_label: &'static str,
    pub icon_globe_label: &'static str,
    pub icon_key_label: &'static str,
    pub icon_chip_label: &'static str,
    pub icon_cluster_label: &'static str,
}

/// 首页快速新增主机表单展示模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct QuickHostViewModel {
    pub group_path: String,
    pub group_options: Vec<GroupOptionViewModel>,
    pub private_key_options: Vec<CredentialOptionViewModel>,
    pub certificate_options: Vec<CredentialOptionViewModel>,
    pub network_proxy_options: Vec<NetworkResourceOptionViewModel>,
    pub network_jump_chain_options: Vec<NetworkResourceOptionViewModel>,
    pub network_forward_options: Vec<NetworkResourceOptionViewModel>,
    pub name: String,
    pub address: String,
    pub port: String,
    pub username: String,
    pub icon_key: String,
    pub tags: String,
    pub auth_kind: &'static str,
    pub agent_source: &'static str,
    pub agent_custom_pipe: String,
    pub password_secret_ref: String,
    pub private_key_ref: String,
    pub passphrase_ref: String,
    pub key_hint: String,
    pub certificate_ref: String,
}
