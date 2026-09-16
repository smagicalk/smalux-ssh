//! 右侧辅助抽屉动态注册模型与 Slint UI 视图数据同步服务。

use std::rc::Rc;
use smagical_core::CoreState;
use slint::ComponentHandle;
use crate::generated::{AppWindow, RightToolBarItemData, WindowBridge};

/// 根据语言环境解析右侧伴生工具栏说明提示 (多语言国际化)
fn resolve_right_panel_tooltip(id: &str, default_tooltip: &str, is_en: bool) -> String {
    if !is_en {
        return default_tooltip.to_string();
    }
    match id {
        "monitor" => "System Monitor",
        "sftp" => "SFTP Transfer",
        "snippets" => "Command Snippets",
        "tmux" => "tmux Session Manager",
        "tunnel" => "Port Forwarding & Tunnels",
        "ai" => "AI Assistant",
        _ => default_tooltip,
    }.to_string()
}

/// 将 CoreState 中的右侧辅助面板注册项同步推送到 Slint UI。
pub fn sync_right_panel_ui(window: &AppWindow, core_state: &CoreState) {
    let is_en = window.global::<WindowBridge>().get_current_language() == "en-US";
    let guard = core_state.right_panels().read().unwrap();
    let items: Vec<RightToolBarItemData> = guard
        .list_visible()
        .into_iter()
        .map(|item| {
            let localized_tooltip = resolve_right_panel_tooltip(&item.id, &item.tooltip, is_en);
            RightToolBarItemData {
                id: item.id.into(),
                icon_name: item.icon_name.into(),
                tooltip: localized_tooltip.into(),
                badge_count: item.badge_count,
                shortcut: item.shortcut.unwrap_or_default().into(),
            }
        })
        .collect();

    let wb = window.global::<WindowBridge>();
    wb.set_right_tool_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(items))));

    tracing::debug!(target: "smagical_ui::right_panel", "右侧面板注册项已同步 (共 {} 项)", guard.list_visible().len());
}
