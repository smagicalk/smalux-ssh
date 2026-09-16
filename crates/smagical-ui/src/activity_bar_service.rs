//! 侧边栏动态注册模型与 Slint UI 视图数据同步服务。

use std::rc::Rc;
use smagical_core::CoreState;

use slint::ComponentHandle;
use crate::generated::{ActivityBarItemData, AppWindow, WindowBridge};

/// 根据语言环境解析活动栏项目说明提示 (多语言国际化)
fn resolve_activity_bar_tooltip(id: &str, default_tooltip: &str, is_en: bool) -> String {
    if !is_en {
        return default_tooltip.to_string();
    }
    match id {
        "hosts" => "Host Inventory",
        "credentials" => "Credentials Vault",
        "history" => "Session History",
        "snippets" => "Command Snippets",
        "tunnel" | "tunnels" => "Network Tunnels",
        "settings" => "Preferences",
        "debug" => "Developer Workbench",
        _ => default_tooltip,
    }.to_string()
}

/// 将 CoreState 中的动态侧边栏注册项同步推送到 Slint UI。
pub fn sync_activity_bar_ui(window: &AppWindow, core_state: &CoreState) {
    let is_en = window.global::<WindowBridge>().get_current_language() == "en-US";

    let top_items: Vec<ActivityBarItemData> = core_state
        .activity_bar()
        .list_top_items()
        .into_iter()
        .map(|item| {
            let localized_tooltip = resolve_activity_bar_tooltip(&item.id, &item.tooltip, is_en);
            ActivityBarItemData {
                id: item.id.into(),
                icon_name: item.icon_name.into(),
                tooltip: localized_tooltip.into(),
                badge_count: item.badge_count,
                shortcut: item.shortcut.unwrap_or_default().into(),
            }
        })
        .collect();

    let bottom_items: Vec<ActivityBarItemData> = core_state
        .activity_bar()
        .list_bottom_items()
        .into_iter()
        .map(|item| {
            let localized_tooltip = resolve_activity_bar_tooltip(&item.id, &item.tooltip, is_en);
            ActivityBarItemData {
                id: item.id.into(),
                icon_name: item.icon_name.into(),
                tooltip: localized_tooltip.into(),
                badge_count: item.badge_count,
                shortcut: item.shortcut.unwrap_or_default().into(),
            }
        })
        .collect();

    let wb = window.global::<WindowBridge>();
    wb.set_top_activity_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(top_items))));
    wb.set_bottom_activity_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(bottom_items))));
}
