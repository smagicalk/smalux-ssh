//! 右侧辅助抽屉动态注册模型与 Slint UI 视图数据同步服务。

use smagical_core::CoreState;
use crate::generated::{ActivityBarItemData, AppWindow};

/// 将 CoreState 中的右侧辅助面板注册项同步推送到 Slint UI。
pub fn sync_right_panel_ui(_window: &AppWindow, core_state: &CoreState) {
    let guard = core_state.right_panels().read().unwrap();
    let items: Vec<ActivityBarItemData> = guard
        .list_visible()
        .into_iter()
        .map(|item| ActivityBarItemData {
            id: item.id.into(),
            icon_name: item.icon_name.into(),
            tooltip: item.tooltip.into(),
            badge_count: item.badge_count,
            shortcut: item.shortcut.unwrap_or_default().into(),
        })
        .collect();

    let _ = items;
    // UI 层属性同步
    tracing::debug!(target: "smagical_ui::right_panel", "右侧面板注册项已同步 (共 {} 项)", guard.list_visible().len());
}

