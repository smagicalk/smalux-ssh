//! 主机列表展示模型。
//!
//! 这里把核心 `StorageManager`、`SessionManager` 和 `UiState` 合成主机首页需要的展示模型。
//! 它不依赖 Slint 类型，因此如果以后重写 UI，主机树、卡片列表、弹窗文案仍可复用。

mod filter;
mod status;
mod types;

#[cfg(test)]
mod tests;

use crate::app::state::{AsDesktopStateView, DesktopStateView};
use crate::model::{
    CredentialKind, CredentialMetadata, GroupId, Host, HostGroup, ProxyProfile, TunnelKind,
    TunnelRule,
};

use super::common::{group_label, host_name, tags_label};
use super::i18n::{Locale, locale_for_state, tr};
use super::labels::auth_label;
use filter::host_matches_query;
use status::{host_status_key, host_status_label};

const TREE_GUIDE_LEVELS: usize = 8;
type TreeGuides = [bool; TREE_GUIDE_LEVELS];

pub(in crate::app) use types::{
    CreateChoiceText, CreateGroupDialogViewModel, CreateHostDialogText, CredentialOptionViewModel,
    GroupOptionViewModel, HostTreeViewModel, HostViewModel, NetworkResourceOptionViewModel,
    QuickHostViewModel,
};

fn accent_index_for_stable_key(key: &str) -> i32 {
    key.bytes()
        .fold(0u32, |acc, byte| {
            acc.wrapping_mul(31).wrapping_add(byte as u32)
        })
        .rem_euclid(5) as i32
}

fn accent_index_for_host_id(host_id: &str) -> i32 {
    // 树形主机节点使用稳定 host_id，避免同一主机在树里每次启动颜色变化。
    accent_index_for_stable_key(host_id)
}

fn accent_index_for_group_id(group_id: Option<GroupId>) -> i32 {
    // 卡片强调色跟随分组；移动主机到其他分组后，左侧标记会随分组更新。
    group_id
        .map(|id| accent_index_for_stable_key(&id.0.to_string()))
        .unwrap_or_default()
}

pub(super) fn hosts(state: impl AsDesktopStateView) -> Vec<HostViewModel> {
    let state = state.as_desktop_state_view();
    // 首页卡片/列表视图使用完整标签展示和分组标题。
    let query = state.ui.workspace.host_search_query.trim().to_lowercase();
    host_rows_for_query(state, &query, true, true)
}

pub(super) fn host_tree(state: impl AsDesktopStateView) -> Vec<HostTreeViewModel> {
    let state = state.as_desktop_state_view();
    // 树视图从一个虚拟 root 开始，空 group_id 表示“所有主机”。
    let query = state.ui.workspace.host_search_query.trim().to_lowercase();
    let locale = locale_for_state(state);
    let root_expanded = !state.ui.workspace.host_tree_root_collapsed;
    let guides = empty_tree_guides();
    let mut rows = vec![HostTreeViewModel {
        id: "root".to_owned(),
        parent_id: String::new(),
        name: tr(locale, "host_tree.root").to_owned(),
        endpoint: String::new(),
        icon_key: "folder".to_owned(),
        kind: "Root",
        host_id: String::new(),
        group_id: String::new(),
        depth: 0,
        expanded: root_expanded,
        status_key: "",
        accent_index: 0,
        has_next_sibling: false,
        guide_0: false,
        guide_1: false,
        guide_2: false,
        guide_3: false,
        guide_4: false,
        guide_5: false,
        guide_6: false,
        guide_7: false,
    }];

    if root_expanded || !query.is_empty() {
        // 搜索时即使分组折叠，也展开匹配路径，保证用户能看到命中结果。
        append_tree_child_rows(state, None, 1, &query, guides, &mut rows);
    }
    rows
}

fn append_tree_child_rows(
    state: DesktopStateView<'_>,
    parent_id: Option<GroupId>,
    depth: i32,
    query: &str,
    guides: TreeGuides,
    rows: &mut Vec<HostTreeViewModel>,
) {
    // 同一层级先显示分组再显示主机，顺序分别在 visible_* 函数中稳定排序。
    let groups = visible_child_groups(state, parent_id, query);
    let hosts = visible_child_hosts(state, parent_id, query);
    let sibling_count = groups.len() + hosts.len();

    for (index, group) in groups.iter().enumerate() {
        let expanded = !state
            .ui
            .workspace
            .collapsed_host_tree_groups
            .contains(&group.id);
        let has_next_sibling = index + 1 < sibling_count;
        rows.push(HostTreeViewModel {
            id: format!("group:{}", group.id.0),
            parent_id: parent_id.map(|id| id.0.to_string()).unwrap_or_default(),
            name: group.name.clone(),
            endpoint: String::new(),
            icon_key: "folder".to_owned(),
            kind: "Group",
            host_id: String::new(),
            group_id: group.id.0.to_string(),
            depth,
            expanded,
            status_key: "",
            accent_index: depth.rem_euclid(5),
            has_next_sibling,
            guide_0: guides[0],
            guide_1: guides[1],
            guide_2: guides[2],
            guide_3: guides[3],
            guide_4: guides[4],
            guide_5: guides[5],
            guide_6: guides[6],
            guide_7: guides[7],
        });

        if expanded || !query.is_empty() {
            // guide 数组描述每一层是否还有后续兄弟节点，Slint 用它画竖线。
            let child_guides = tree_guides_with_depth(guides, depth, has_next_sibling);
            append_tree_child_rows(state, Some(group.id), depth + 1, query, child_guides, rows);
        }
    }

    for (offset, host) in hosts.iter().enumerate() {
        let id = host.id.0.to_string();
        let index = groups.len() + offset;
        let has_next_sibling = index + 1 < sibling_count;
        rows.push(HostTreeViewModel {
            id: format!("host:{id}"),
            parent_id: parent_id.map(|id| id.0.to_string()).unwrap_or_default(),
            name: host.name.clone(),
            endpoint: format!("{}:{}", host.address, host.port),
            icon_key: host.icon_key.clone(),
            kind: "Host",
            host_id: id.clone(),
            group_id: parent_id.map(|id| id.0.to_string()).unwrap_or_default(),
            depth,
            expanded: false,
            status_key: host_status_key(state, host.id),
            accent_index: accent_index_for_host_id(&id),
            has_next_sibling,
            guide_0: guides[0],
            guide_1: guides[1],
            guide_2: guides[2],
            guide_3: guides[3],
            guide_4: guides[4],
            guide_5: guides[5],
            guide_6: guides[6],
            guide_7: guides[7],
        });
    }
}

fn visible_child_groups<'a>(
    state: DesktopStateView<'_>,
    parent_id: Option<GroupId>,
    query: &str,
) -> Vec<HostGroup> {
    // 分组自己命中或子孙命中时都要保留，这样搜索结果能显示完整路径。
    let mut groups = state
        .storage
        .groups
        .iter()
        .filter(|group| group.parent_id == parent_id)
        .filter(|group| {
            let group_matches = query.is_empty() || group.name.to_lowercase().contains(query);
            group_matches || group_has_matching_descendant(state, group.id, query)
        })
        .cloned()
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| left.name.cmp(&right.name));
    groups
}

fn visible_child_hosts<'a>(
    state: DesktopStateView<'_>,
    group_id: Option<GroupId>,
    query: &str,
) -> Vec<Host> {
    // 只返回当前父分组下的主机；递归结构由 append_tree_child_rows 控制。
    let mut hosts = state
        .storage
        .hosts
        .iter()
        .filter(|host| host.group_id == group_id && host_matches_query(host, query))
        .cloned()
        .collect::<Vec<_>>();
    hosts.sort_by(|left, right| left.name.cmp(&right.name));
    hosts
}

fn empty_tree_guides() -> TreeGuides {
    [false; TREE_GUIDE_LEVELS]
}

fn tree_guides_with_depth(mut guides: TreeGuides, depth: i32, active: bool) -> TreeGuides {
    // 超出预设层级时不继续写 guide，避免异常深层分组导致数组越界。
    let Ok(index) = usize::try_from(depth) else {
        return guides;
    };
    if index < TREE_GUIDE_LEVELS {
        guides[index] = active;
    }
    guides
}

fn group_has_matching_descendant(
    state: DesktopStateView<'_>,
    group_id: GroupId,
    query: &str,
) -> bool {
    // 空查询时所有分组都可见；非空查询递归检查子主机和子分组。
    query.is_empty()
        || state
            .storage
            .hosts
            .iter()
            .any(|host| host.group_id == Some(group_id) && host_matches_query(host, query))
        || state
            .storage
            .groups
            .iter()
            .filter(|group| group.parent_id == Some(group_id))
            .any(|group| {
                group.name.to_lowercase().contains(query)
                    || group_has_matching_descendant(state, group.id, query)
            })
}

pub(super) fn new_session_hosts(state: impl AsDesktopStateView) -> Vec<HostViewModel> {
    let state = state.as_desktop_state_view();
    // 新建会话弹窗使用紧凑标签，避免卡片高度被长 tag 撑大。
    let query = state
        .ui
        .workspace
        .new_session_search_query
        .trim()
        .to_lowercase();
    host_rows_for_query(state, "", false, false)
        .into_iter()
        .filter(|host| host_view_matches_query(host, &query))
        .collect()
}

fn host_rows_for_query(
    state: DesktopStateView<'_>,
    query: &str,
    include_group_headers: bool,
    show_full_tags: bool,
) -> Vec<HostViewModel> {
    let locale = locale_for_state(state);
    let mut last_group = String::new();
    let mut last_group_id = String::new();

    state
        .storage
        .hosts
        .iter()
        .filter(|host| host_matches_query(host, &query))
        .map(|host| {
            // group_header 只在分组变化时填充，Slint 可以按行决定是否显示分隔标题。
            let group = group_label(state, host);
            let group_id = host.group_id.map(|id| id.0.to_string()).unwrap_or_default();
            let group_header = if !include_group_headers || group == last_group {
                String::new()
            } else {
                last_group = group.clone();
                last_group_id = group_id.clone();
                group.clone()
            };
            let group_header_id = if group_header.is_empty() {
                String::new()
            } else {
                last_group_id.clone()
            };

            HostViewModel {
                id: host.id.0.to_string(),
                name: host.name.clone(),
                endpoint: format!("{}:{}", host.address, host.port),
                icon_key: host.icon_key.clone(),
                auth: auth_label(&host.auth, locale),
                group,
                group_id,
                group_header,
                group_header_id,
                tags: if show_full_tags {
                    tag_display::full(state, host)
                } else {
                    tag_display::compact(state, host)
                },
                status_key: host_status_key(state, host.id),
                status: host_status_label(state, host.id, locale),
                accent_index: accent_index_for_group_id(host.group_id),
            }
        })
        .collect()
}

pub(super) fn create_group_dialog(state: impl AsDesktopStateView) -> CreateGroupDialogViewModel {
    let state = state.as_desktop_state_view();
    // 创建分组和选择父级共用一份文案模型，实际显示哪个弹窗由 workspace 状态决定。
    let locale = locale_for_state(state);
    let default_group = tr(locale, "common.ungrouped");

    CreateGroupDialogViewModel {
        title: tr(locale, "group.create_title"),
        subtitle: tr(locale, "group.create_subtitle"),
        parent_picker_title: tr(locale, "group.parent_picker_title"),
        parent_picker_subtitle: tr(locale, "group.parent_picker_subtitle"),
        name_label: tr(locale, "group.name_label"),
        name_placeholder: tr(locale, "group.name_placeholder"),
        parent_label: tr(locale, "group.parent_label"),
        cancel_label: tr(locale, "host.cancel"),
        save_label: tr(locale, "group.create_confirm"),
        next_label: tr(locale, "group.parent_picker_next"),
        parent_path: group_path(
            state,
            effective_create_group_parent_id(state),
            default_group,
        ),
        parent_options: group_options(
            state,
            default_group,
            effective_create_group_parent_id(state),
        ),
        name: state.ui.quick_group.name.clone(),
    }
}

fn effective_create_group_parent_id(state: DesktopStateView<'_>) -> Option<crate::model::GroupId> {
    // 父级选择弹窗打开时，pending 选择优先于真正的 quick_group 草稿。
    if state.ui.workspace.create_group_parent_dialog_open {
        state.ui.workspace.pending_create_group_parent_id
    } else {
        state.ui.quick_group.parent_id
    }
}

pub(super) fn create_choice_text(state: impl AsDesktopStateView) -> CreateChoiceText {
    let state = state.as_desktop_state_view();
    // 创建入口弹窗只有文案，不持有业务状态。
    let locale = locale_for_state(state);

    CreateChoiceText {
        title: tr(locale, "create_choice.title"),
        subtitle: tr(locale, "create_choice.subtitle"),
        host_label: tr(locale, "create_choice.host_label"),
        host_caption: tr(locale, "create_choice.host_caption"),
        group_label: tr(locale, "create_choice.group_label"),
        group_caption: tr(locale, "create_choice.group_caption"),
        cancel_label: tr(locale, "host.cancel"),
    }
}

fn host_view_matches_query(host: &HostViewModel, query: &str) -> bool {
    // 新建会话弹窗对已经投影后的字段搜索，能匹配本地化后的认证/状态文本。
    query.is_empty()
        || host.name.to_lowercase().contains(query)
        || host.endpoint.to_lowercase().contains(query)
        || host.auth.to_lowercase().contains(query)
        || host.group.to_lowercase().contains(query)
        || host.tags.to_lowercase().contains(query)
        || host.status.to_lowercase().contains(query)
}

mod tag_display {
    use crate::app::state::AsDesktopStateView;
    use crate::model::Host;

    use super::tags_label;

    pub(super) fn full(state: impl AsDesktopStateView, host: &Host) -> String {
        tags_label(state, host)
    }

    pub(super) fn compact(_state: impl AsDesktopStateView, host: &Host) -> String {
        // 弹窗卡片只显示第一个 tag 和剩余数量，减少横向挤压。
        compact_host_tags(&host.tags)
    }

    fn compact_host_tags(tags: &[String]) -> String {
        let Some(first) = tags.first() else {
            return String::new();
        };
        let remaining = tags.len().saturating_sub(1);

        if remaining == 0 {
            first.to_owned()
        } else {
            format!("{first} +{remaining}")
        }
    }
}

pub(super) fn quick_host(state: impl AsDesktopStateView) -> QuickHostViewModel {
    let state = state.as_desktop_state_view();
    // 草稿字段原样回填，认证方式和 agent source 使用稳定 key 供 Slint 单选控件匹配。
    let draft = &state.ui.quick_host;
    let locale = locale_for_state(state);
    let default_group = tr(locale, "common.ungrouped");

    QuickHostViewModel {
        group_path: group_path(state, draft.group_id, default_group),
        group_options: group_options(state, default_group, draft.group_id),
        private_key_options: credential_options(
            state,
            CredentialKind::PrivateKey,
            &draft.auth.private_key_ref,
        ),
        certificate_options: credential_options(
            state,
            CredentialKind::Certificate,
            &draft.auth.certificate_ref,
        ),
        network_proxy_options: network_proxy_options(state),
        network_jump_chain_options: network_jump_chain_options(state),
        network_forward_options: network_forward_options(state),
        name: draft.name.clone(),
        address: draft.address.clone(),
        port: draft.port.clone(),
        username: draft.username.clone(),
        icon_key: draft.icon_key.clone(),
        tags: draft.tags.clone(),
        auth_kind: draft.auth.kind.label(),
        agent_source: draft.auth.agent_source.label(),
        agent_custom_pipe: draft.auth.agent_custom_pipe.clone(),
        password_secret_ref: draft.auth.password_secret_ref.clone(),
        private_key_ref: draft.auth.private_key_ref.clone(),
        passphrase_ref: draft.auth.passphrase_ref.clone(),
        key_hint: draft.auth.key_hint.clone(),
        certificate_ref: draft.auth.certificate_ref.clone(),
    }
}

fn credential_options(
    state: DesktopStateView<'_>,
    kind: CredentialKind,
    selected_value: &str,
) -> Vec<CredentialOptionViewModel> {
    let mut rows = state
        .storage
        .credentials
        .iter()
        .filter(|credential| credential.kind == kind)
        .filter_map(|credential| {
            credential
                .secret
                .as_ref()
                .map(|secret| (credential, secret))
        })
        .map(|(credential, secret)| {
            let value = secret.0.clone();
            CredentialOptionViewModel {
                value: value.clone(),
                label: credential.name.clone(),
                detail: credential_detail(credential),
                selected: value == selected_value,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.label.cmp(&right.label));
    rows
}

fn credential_detail(credential: &CredentialMetadata) -> String {
    credential
        .username
        .clone()
        .or_else(|| credential.fingerprint.clone())
        .or_else(|| credential.key_algorithm.as_ref().map(key_algorithm_label))
        .unwrap_or_else(|| credential.name.clone())
}

fn key_algorithm_label(algorithm: &crate::model::KeyAlgorithm) -> String {
    match algorithm {
        crate::model::KeyAlgorithm::Ed25519 => "ed25519".to_owned(),
        crate::model::KeyAlgorithm::Rsa => "rsa".to_owned(),
        crate::model::KeyAlgorithm::Ecdsa => "ecdsa".to_owned(),
        crate::model::KeyAlgorithm::Unknown(name) => name.clone(),
    }
}

fn network_proxy_options(state: DesktopStateView<'_>) -> Vec<NetworkResourceOptionViewModel> {
    let locale = locale_for_state(state);
    let selected_ids = &state.ui.quick_host.network.proxy_ids;
    let mut rows = state
        .storage
        .proxy_assets
        .iter()
        .map(|asset| NetworkResourceOptionViewModel {
            value: asset.id.0.to_string(),
            label: asset.name.clone(),
            detail: proxy_profile_label(&asset.profile),
            kind_key: "ProxyAsset",
            kind_label: tr(locale, "host.network_proxy_label").to_owned(),
            icon_key: "globe",
            accent_index: 1,
            selected: selected_ids.contains(&asset.id),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.label.cmp(&right.label));
    rows
}

fn network_jump_chain_options(state: DesktopStateView<'_>) -> Vec<NetworkResourceOptionViewModel> {
    let locale = locale_for_state(state);
    let selected_ids = &state.ui.quick_host.network.jump_chain_ids;
    let mut rows = state
        .storage
        .jump_chain_assets
        .iter()
        .map(|asset| {
            let path = asset
                .steps
                .iter()
                .map(|step| host_name(state, step.host_id))
                .collect::<Vec<_>>();
            NetworkResourceOptionViewModel {
                value: asset.id.0.to_string(),
                label: asset.name.clone(),
                detail: if path.is_empty() {
                    tr(locale, "tool.empty_value").to_owned()
                } else {
                    path.join(" -> ")
                },
                kind_key: "JumpChainAsset",
                kind_label: tr(locale, "host.network_jump_label").to_owned(),
                icon_key: "router",
                accent_index: 2,
                selected: selected_ids.contains(&asset.id),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.label.cmp(&right.label));
    rows
}

fn network_forward_options(state: DesktopStateView<'_>) -> Vec<NetworkResourceOptionViewModel> {
    let locale = locale_for_state(state);
    let selected_ids = &state.ui.quick_host.network.forward_ids;
    let mut rows = state
        .storage
        .forward_assets
        .iter()
        .map(|asset| NetworkResourceOptionViewModel {
            value: asset.id.0.to_string(),
            label: asset.name.clone(),
            detail: forward_rule_label(&asset.rule, locale),
            kind_key: "ForwardAsset",
            kind_label: tr(locale, "host.network_forward_label").to_owned(),
            icon_key: "router",
            accent_index: 3,
            selected: selected_ids.contains(&asset.id),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.label.cmp(&right.label));
    rows
}

fn proxy_profile_label(profile: &ProxyProfile) -> String {
    match profile {
        ProxyProfile::Socks5 { host, port, .. } => format!("SOCKS5 {host}:{port}"),
        ProxyProfile::Http { host, port, .. } => format!("HTTP {host}:{port}"),
    }
}

fn forward_rule_label(rule: &TunnelRule, locale: Locale) -> String {
    match rule.kind {
        TunnelKind::Local => format!(
            "{} {}:{} -> {}:{}",
            tr(locale, "host.network_forward_local"),
            rule.bind_host,
            rule.bind_port,
            rule.target_host,
            rule.target_port
        ),
        TunnelKind::Remote => format!(
            "{} {}:{} -> {}:{}",
            tr(locale, "host.network_forward_remote"),
            rule.bind_host,
            rule.bind_port,
            rule.target_host,
            rule.target_port
        ),
        TunnelKind::Dynamic => format!(
            "{} {}:{}",
            tr(locale, "host.network_forward_dynamic"),
            rule.bind_host,
            rule.bind_port
        ),
    }
}

fn group_options(
    state: DesktopStateView<'_>,
    default_group: &'static str,
    selected_group_id: Option<GroupId>,
) -> Vec<GroupOptionViewModel> {
    // 根分组不是实体分组，所以使用空 id；保存时会转换为 None。
    let mut rows = vec![GroupOptionViewModel {
        id: String::new(),
        name: default_group.to_owned(),
        path: default_group.to_owned(),
        depth: 0,
        selected: selected_group_id.is_none(),
    }];
    append_group_options(state, None, 0, Vec::new(), selected_group_id, &mut rows);
    rows
}

fn append_group_options(
    state: DesktopStateView<'_>,
    parent_id: Option<GroupId>,
    depth: i32,
    parent_path: Vec<String>,
    selected_group_id: Option<GroupId>,
    rows: &mut Vec<GroupOptionViewModel>,
) {
    let mut groups = state
        .storage
        .groups
        .iter()
        .filter(|group| group.parent_id == parent_id)
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| left.name.cmp(&right.name));

    for group in groups {
        // path 用于弹窗中展示完整层级，name 用于行内主标题。
        let mut path = parent_path.clone();
        path.push(group.name.clone());
        let path_text = path.join(" / ");
        rows.push(GroupOptionViewModel {
            id: group.id.0.to_string(),
            name: group.name.clone(),
            path: path_text,
            depth,
            selected: selected_group_id == Some(group.id),
        });
        append_group_options(
            state,
            Some(group.id),
            depth + 1,
            path,
            selected_group_id,
            rows,
        );
    }
}

fn group_path(
    state: DesktopStateView<'_>,
    group_id: Option<GroupId>,
    default_group: &'static str,
) -> String {
    // None 或已删除分组都回退到根分组文案，避免 UI 显示裸 UUID。
    let Some(group_id) = group_id else {
        return default_group.to_owned();
    };

    let Some(group) = find_group(state, group_id) else {
        return default_group.to_owned();
    };

    let mut names = vec![group.name.clone()];
    let mut parent_id = group.parent_id;
    while let Some(id) = parent_id {
        let Some(parent) = find_group(state, id) else {
            break;
        };
        names.push(parent.name.clone());
        parent_id = parent.parent_id;
    }
    names.reverse();
    names.join(" / ")
}

fn find_group(state: DesktopStateView<'_>, group_id: GroupId) -> Option<HostGroup> {
    state
        .storage
        .groups
        .iter()
        .find(|group| group.id == group_id)
        .cloned()
}

pub(super) fn create_host_dialog_text(state: impl AsDesktopStateView) -> CreateHostDialogText {
    let state = state.as_desktop_state_view();
    // 创建和编辑共用一个弹窗，只根据 editing 状态切换标题和确认按钮文案。
    let editing = state.ui.quick_host.editing_host_id.is_some();
    let locale = locale_for_state(state);

    CreateHostDialogText {
        editing,
        dialog_title: if editing {
            tr(locale, "host.edit_title")
        } else {
            tr(locale, "host.create_title")
        },
        dialog_subtitle: if editing {
            tr(locale, "host.edit_subtitle")
        } else {
            tr(locale, "host.create_subtitle")
        },
        connection_title: tr(locale, "host.connection_title"),
        connection_caption: tr(locale, "host.connection_caption"),
        authentication_title: tr(locale, "host.authentication_title"),
        authentication_caption: tr(locale, "host.authentication_caption"),
        credential_note: tr(locale, "host.credential_note"),
        cancel_label: tr(locale, "host.cancel"),
        create_label: if editing {
            tr(locale, "host.save_confirm")
        } else {
            tr(locale, "host.create_confirm")
        },
        address_label: tr(locale, "host.address_label"),
        address_placeholder: tr(locale, "host.address_placeholder"),
        port_label: tr(locale, "host.port_label"),
        port_placeholder: tr(locale, "host.port_placeholder"),
        username_label: tr(locale, "host.username_label"),
        username_placeholder: tr(locale, "host.username_placeholder"),
        host_alias_label: tr(locale, "host.alias_label"),
        host_alias_placeholder: tr(locale, "host.alias_placeholder"),
        tags_label: tr(locale, "host.tags_label"),
        tags_placeholder: tr(locale, "host.tags_placeholder"),
        group_label: tr(locale, "host.group_label"),
        group_picker_title: tr(locale, "host.group_picker_title"),
        auth_agent_label: tr(locale, "auth.agent"),
        auth_agent_caption: tr(locale, "host.auth_agent_caption"),
        auth_password_label: tr(locale, "auth.password"),
        auth_password_caption: tr(locale, "host.auth_password_caption"),
        auth_key_label: tr(locale, "auth.key"),
        auth_key_caption: tr(locale, "host.auth_key_caption"),
        auth_certificate_label: tr(locale, "auth.certificate"),
        auth_certificate_caption: tr(locale, "host.auth_certificate_caption"),
        agent_source_title: tr(locale, "host.agent_source_title"),
        agent_auto_label: tr(locale, "host.agent_auto_label"),
        agent_auto_caption: tr(locale, "host.agent_auto_caption"),
        agent_openssh_label: tr(locale, "host.agent_openssh_label"),
        agent_openssh_caption: tr(locale, "host.agent_openssh_caption"),
        agent_pageant_label: tr(locale, "host.agent_pageant_label"),
        agent_pageant_caption: tr(locale, "host.agent_pageant_caption"),
        agent_custom_label: tr(locale, "host.agent_custom_label"),
        agent_custom_caption: tr(locale, "host.agent_custom_caption"),
        custom_agent_pipe_label: tr(locale, "host.custom_agent_pipe_label"),
        custom_agent_pipe_placeholder: tr(locale, "host.custom_agent_pipe_placeholder"),
        agent_key_hint_label: tr(locale, "host.agent_key_hint_label"),
        agent_key_hint_placeholder: tr(locale, "host.agent_key_hint_placeholder"),
        password_secret_label: tr(locale, "host.password_secret_label"),
        password_secret_placeholder: tr(locale, "host.password_secret_placeholder"),
        private_key_label: tr(locale, "host.private_key_label"),
        private_key_placeholder: tr(locale, "host.private_key_placeholder"),
        add_private_key_label: tr(locale, "host.add_private_key"),
        add_private_key_caption: tr(locale, "host.add_private_key_caption"),
        passphrase_label: tr(locale, "host.passphrase_label"),
        passphrase_placeholder: tr(locale, "host.passphrase_placeholder"),
        certificate_label: tr(locale, "host.certificate_label"),
        certificate_placeholder: tr(locale, "host.certificate_placeholder"),
        add_certificate_label: tr(locale, "host.add_certificate"),
        add_certificate_caption: tr(locale, "host.add_certificate_caption"),
        credential_name_label: tr(locale, "host.credential_name_label"),
        credential_name_placeholder: tr(locale, "host.credential_name_placeholder"),
        credential_secret_label: tr(locale, "host.credential_secret_label"),
        credential_secret_placeholder: tr(locale, "host.credential_secret_placeholder"),
        credential_algorithm_label: tr(locale, "host.credential_algorithm_label"),
        credential_save_label: tr(locale, "host.credential_save"),
        network_title: tr(locale, "host.network_title"),
        network_caption: tr(locale, "host.network_caption"),
        network_proxy_label: tr(locale, "host.network_proxy_label"),
        network_jump_label: tr(locale, "host.network_jump_label"),
        network_forward_label: tr(locale, "host.network_forward_label"),
        network_empty_label: tr(locale, "host.network_empty_label"),
        icon_title: tr(locale, "host.icon_title"),
        icon_server_label: tr(locale, "host.icon_server"),
        icon_database_label: tr(locale, "host.icon_database"),
        icon_cloud_label: tr(locale, "host.icon_cloud"),
        icon_linux_label: tr(locale, "host.icon_linux"),
        icon_container_label: tr(locale, "host.icon_container"),
        icon_shield_label: tr(locale, "host.icon_shield"),
        icon_router_label: tr(locale, "host.icon_router"),
        icon_terminal_label: tr(locale, "host.icon_terminal"),
        icon_globe_label: tr(locale, "host.icon_globe"),
        icon_key_label: tr(locale, "host.icon_key"),
        icon_chip_label: tr(locale, "host.icon_chip"),
        icon_cluster_label: tr(locale, "host.icon_cluster"),
    }
}
