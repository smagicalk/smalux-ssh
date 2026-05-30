//! 设置页 Slint 模型转换。
//!
//! 设置页数据量不大，但类型多：语言/主题选项、主题 profile、存储摘要、导入导出动作。
//! 这里把 view model 转成 Slint Row，保持 `.slint` 文件不需要理解 Rust 枚举。

use slint::{ModelRc, VecModel};

use crate::app::view_model::{
    CustomThemeProfileViewModel, SettingOptionViewModel, SettingsFileActionViewModel,
    SettingsStorageSummaryItemViewModel,
};
use crate::app::{SettingOptionRow, SettingsFileActionRow, SettingsProfileRow, SettingsSummaryRow};

pub(in crate::app::projection) fn setting_option_model(
    items: &[SettingOptionViewModel],
) -> ModelRc<SettingOptionRow> {
    // 通用选项行用于语言和内置主题，key 是回调协议值，label 是当前语言文案。
    let rows = items
        .iter()
        .map(|item| SettingOptionRow {
            key: item.key.into(),
            label: item.label.into(),
            selected: item.selected,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn settings_profile_model(
    items: &[CustomThemeProfileViewModel],
) -> ModelRc<SettingsProfileRow> {
    // 自定义主题 profile 可以应用或删除，内置主题导出的 profile 不允许删除。
    let rows = items
        .iter()
        .map(|item| SettingsProfileRow {
            name: item.name.as_str().into(),
            source_label: item.source_label.into(),
            selected: item.selected,
            can_apply: item.can_apply,
            can_remove: item.can_remove,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn settings_summary_model(
    items: &[SettingsStorageSummaryItemViewModel],
) -> ModelRc<SettingsSummaryRow> {
    // 存储摘要只展示计数，不暴露具体数据内容。
    let rows = items
        .iter()
        .map(|item| SettingsSummaryRow {
            key: item.key.into(),
            label: item.label.into(),
            count: item.count as i32,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn settings_file_action_model(
    items: &[SettingsFileActionViewModel],
) -> ModelRc<SettingsFileActionRow> {
    // 文件操作行包含方向、格式、默认文件名和启用状态，按钮行为由回调层根据 key 分发。
    let rows = items
        .iter()
        .map(|item| SettingsFileActionRow {
            key: item.key.into(),
            label: item.label.into(),
            category_key: item.category_key.into(),
            category_label: item.category_label.into(),
            direction: item.direction.into(),
            direction_label: item.direction_label.into(),
            format_key: item.format_key.into(),
            format_label: item.format_label.into(),
            default_file_name: item.default_file_name.as_str().into(),
            default_extension: item.default_extension.into(),
            path_placeholder: item.path_placeholder.into(),
            enabled: item.enabled,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
