//! 侧边栏动态注册模型与 Slint UI 视图数据同步服务。

use std::rc::Rc;
use smagical_core::CoreState;


use crate::generated::{ActivityBarItemData, AppWindow};

/// 将 CoreState 中的动态侧边栏注册项同步推送到 Slint UI。
pub fn sync_activity_bar_ui(window: &AppWindow, core_state: &CoreState) {
    let top_items: Vec<ActivityBarItemData> = core_state
        .activity_bar()
        .list_top_items()
        .into_iter()
        .map(|item| ActivityBarItemData {
            id: item.id.into(),
            icon_name: item.icon_name.into(),
            tooltip: item.tooltip.into(),
            badge_count: item.badge_count,
            shortcut: item.shortcut.unwrap_or_default().into(),
        })
        .collect();

    let bottom_items: Vec<ActivityBarItemData> = core_state
        .activity_bar()
        .list_bottom_items()
        .into_iter()
        .map(|item| ActivityBarItemData {
            id: item.id.into(),
            icon_name: item.icon_name.into(),
            tooltip: item.tooltip.into(),
            badge_count: item.badge_count,
            shortcut: item.shortcut.unwrap_or_default().into(),
        })
        .collect();

    window.set_top_activity_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(top_items))));
    window.set_bottom_activity_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(bottom_items))));
}
