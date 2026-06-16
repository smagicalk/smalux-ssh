//! 主机列表 Slint 模型转换。
//!
//! `view_model::hosts` 输出 Rust 结构体，Slint 需要 `.slint` 中声明的 Row 类型。这里只做
//! 字段拷贝和字符串转换，不做过滤、排序或本地化。

use slint::{ModelRc, VecModel};

use crate::app::view_model::{
    CredentialOptionViewModel, GroupOptionViewModel, HostTreeViewModel, HostViewModel,
    NetworkResourceOptionViewModel,
};
use crate::app::{
    CredentialOptionRow, GroupOptionRow, HostRow, HostTreeRow, NetworkResourceOptionRow,
};

pub(in crate::app::projection) fn host_model(items: &[HostViewModel]) -> ModelRc<HostRow> {
    // HostRow 对应卡片/列表视图，包含完整 endpoint、auth、tags 和状态展示字段。
    let rows = items
        .iter()
        .map(|host| HostRow {
            id: host.id.as_str().into(),
            name: host.name.as_str().into(),
            endpoint: host.endpoint.as_str().into(),
            icon_key: host.icon_key.as_str().into(),
            auth: host.auth.into(),
            group: host.group.as_str().into(),
            group_id: host.group_id.as_str().into(),
            group_header: host.group_header.as_str().into(),
            group_header_id: host.group_header_id.as_str().into(),
            tags: host.tags.as_str().into(),
            network_summary: host.network_summary.as_str().into(),
            status_key: host.status_key.into(),
            status: host.status.into(),
            accent_index: host.accent_index,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn host_tree_model(
    items: &[HostTreeViewModel],
) -> ModelRc<HostTreeRow> {
    // HostTreeRow 对应文件树视图，额外包含 depth、guide 和展开状态。
    let rows = items
        .iter()
        .map(|item| HostTreeRow {
            id: item.id.as_str().into(),
            parent_id: item.parent_id.as_str().into(),
            name: item.name.as_str().into(),
            endpoint: item.endpoint.as_str().into(),
            icon_key: item.icon_key.as_str().into(),
            kind: item.kind.into(),
            host_id: item.host_id.as_str().into(),
            group_id: item.group_id.as_str().into(),
            depth: item.depth,
            expanded: item.expanded,
            status_key: item.status_key.into(),
            accent_index: item.accent_index,
            has_next_sibling: item.has_next_sibling,
            guide_0: item.guide_0,
            guide_1: item.guide_1,
            guide_2: item.guide_2,
            guide_3: item.guide_3,
            guide_4: item.guide_4,
            guide_5: item.guide_5,
            guide_6: item.guide_6,
            guide_7: item.guide_7,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn group_option_model(
    items: &[GroupOptionViewModel],
) -> ModelRc<GroupOptionRow> {
    // 分组选项被创建主机、创建分组父级选择复用。
    let rows = items
        .iter()
        .map(|group| GroupOptionRow {
            id: group.id.as_str().into(),
            name: group.name.as_str().into(),
            path: group.path.as_str().into(),
            depth: group.depth,
            selected: group.selected,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn credential_option_model(
    items: &[CredentialOptionViewModel],
) -> ModelRc<CredentialOptionRow> {
    let rows = items
        .iter()
        .map(|item| CredentialOptionRow {
            value: item.value.as_str().into(),
            label: item.label.as_str().into(),
            detail: item.detail.as_str().into(),
            selected: item.selected,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn network_resource_option_model(
    items: &[NetworkResourceOptionViewModel],
) -> ModelRc<NetworkResourceOptionRow> {
    let rows = items
        .iter()
        .map(|item| NetworkResourceOptionRow {
            value: item.value.as_str().into(),
            label: item.label.as_str().into(),
            detail: item.detail.as_str().into(),
            kind_key: item.kind_key.into(),
            kind_label: item.kind_label.as_str().into(),
            icon_key: item.icon_key.into(),
            accent_index: item.accent_index,
            selected: item.selected,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
