//! 通用工具列表 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::view_model::{NetworkNavItemViewModel, ToolItemViewModel};
use crate::app::{NetworkItemRow, ToolItemRow};

pub(in crate::app::projection) fn tool_item_model(
    items: &[ToolItemViewModel],
) -> ModelRc<ToolItemRow> {
    let rows = items
        .iter()
        .map(|item| ToolItemRow {
            title: item.title.as_str().into(),
            subtitle: item.subtitle.as_str().into(),
            meta: item.meta.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn network_item_model(
    items: &[NetworkNavItemViewModel],
) -> ModelRc<NetworkItemRow> {
    let rows = items
        .iter()
        .map(|item| NetworkItemRow {
            id: item.id.as_str().into(),
            title: item.title.as_str().into(),
            subtitle: item.subtitle.as_str().into(),
            meta: item.meta.as_str().into(),
            kind_key: item.kind_key.into(),
            kind_label: item.kind_label.as_str().into(),
            note: item.note.as_str().into(),
            icon_key: item.icon_key.into(),
            accent_index: item.accent_index,
            session_id: item.session_id.as_str().into(),
            primary_action_key: item.primary_action_key.into(),
            primary_action_label: item.primary_action_label.as_str().into(),
            primary_action_enabled: item.primary_action_enabled,
            stat_primary_label: item.stat_primary_label.as_str().into(),
            stat_primary_value: item.stat_primary_value.as_str().into(),
            stat_secondary_label: item.stat_secondary_label.as_str().into(),
            stat_secondary_value: item.stat_secondary_value.as_str().into(),
            detail_primary_label: item.detail_primary_label.as_str().into(),
            detail_primary_value: item.detail_primary_value.as_str().into(),
            detail_secondary_label: item.detail_secondary_label.as_str().into(),
            detail_secondary_value: item.detail_secondary_value.as_str().into(),
            body_label: item.body_label.as_str().into(),
            body_value: item.body_value.as_str().into(),
            asset_id: item.asset_id.as_str().into(),
            edit_kind_key: item.edit_kind_key.as_str().into(),
            edit_host: item.edit_host.as_str().into(),
            edit_port: item.edit_port.as_str().into(),
            edit_tags: item.edit_tags.as_str().into(),
            edit_bind_host: item.edit_bind_host.as_str().into(),
            edit_bind_port: item.edit_bind_port.as_str().into(),
            edit_target_host: item.edit_target_host.as_str().into(),
            edit_target_port: item.edit_target_port.as_str().into(),
            edit_auto_start: item.edit_auto_start,
            edit_host_ids: item.edit_host_ids.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
