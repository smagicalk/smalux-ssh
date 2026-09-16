//! 快捷键配置、格式化与键位映射事件处理器

use std::cell::RefCell;
use std::rc::Rc;
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::generated::{AppWindow, SettingKeybindingItem, SettingsBridge};
use crate::handlers::AppContext;

/// 获取系统默认官方快捷键配置清单
pub fn get_default_keybindings() -> Vec<SettingKeybindingItem> {
    vec![
        SettingKeybindingItem {
            id: "term.new_tab".into(),
            action_name: "新建终端标签页".into(),
            category: "tabs".into(),
            primary_key: "Ctrl+T".into(),
            secondary_key: "Ctrl+Shift+T".into(),
            default_key: "Ctrl+T".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "term.close_tab".into(),
            action_name: "关闭当前终端标签页".into(),
            category: "tabs".into(),
            primary_key: "Ctrl+W".into(),
            secondary_key: "".into(),
            default_key: "Ctrl+W".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "term.split_horizontal".into(),
            action_name: "水平分屏 (左右切分窗格)".into(),
            category: "panes".into(),
            primary_key: "Alt+Shift+D".into(),
            secondary_key: "".into(),
            default_key: "Alt+Shift+D".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "term.split_vertical".into(),
            action_name: "垂直分屏 (上下切分窗格)".into(),
            category: "panes".into(),
            primary_key: "Alt+Shift+E".into(),
            secondary_key: "".into(),
            default_key: "Alt+Shift+E".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "term.copy".into(),
            action_name: "复制终端选中文本".into(),
            category: "terminal".into(),
            primary_key: "Ctrl+Shift+C".into(),
            secondary_key: "".into(),
            default_key: "Ctrl+Shift+C".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "term.paste".into(),
            action_name: "粘贴剪贴板内容到终端".into(),
            category: "terminal".into(),
            primary_key: "Ctrl+Shift+V".into(),
            secondary_key: "".into(),
            default_key: "Ctrl+Shift+V".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "term.clear".into(),
            action_name: "清除当前终端屏幕".into(),
            category: "terminal".into(),
            primary_key: "Ctrl+L".into(),
            secondary_key: "".into(),
            default_key: "Ctrl+L".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "term.find".into(),
            action_name: "查找终端屏幕内容".into(),
            category: "terminal".into(),
            primary_key: "Ctrl+F".into(),
            secondary_key: "".into(),
            default_key: "Ctrl+F".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "general.command_palette".into(),
            action_name: "打开全局命令搜索面板".into(),
            category: "general".into(),
            primary_key: "Ctrl+P".into(),
            secondary_key: "Ctrl+K".into(),
            default_key: "Ctrl+P".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "general.settings".into(),
            action_name: "打开偏好设置中心".into(),
            category: "general".into(),
            primary_key: "Ctrl+,".into(),
            secondary_key: "".into(),
            default_key: "Ctrl+,".into(),
            is_customized: false,
        },
        SettingKeybindingItem {
            id: "debug.toggle_workbench".into(),
            action_name: "打开/关闭开发者调试控制台".into(),
            category: "debug".into(),
            primary_key: "F12".into(),
            secondary_key: "".into(),
            default_key: "F12".into(),
            is_customized: false,
        },
    ]
}

pub(crate) fn register_keybinding_handlers(window: &AppWindow, ctx: &AppContext) {
    let bridge = window.global::<SettingsBridge>();

    let keys_state = Rc::new(RefCell::new(get_default_keybindings()));
    bridge.set_keybindings(ModelRc::from(Rc::new(VecModel::from(keys_state.borrow().clone()))));

    let keys_ref_up = keys_state.clone();
    let window_weak_k_up = window.as_weak();
    let notif_k_up = ctx.notifications.clone();
    bridge.on_update_keybinding(move |id, new_key| {
        let mut list = keys_ref_up.borrow_mut();
        if let Some(item) = list.iter_mut().find(|it| it.id == id) {
            item.primary_key = new_key.clone();
            item.is_customized = item.primary_key != item.default_key;
            notif_k_up.success("快捷键已更新", &format!("「{}」快捷键已更新为 {}", item.action_name, new_key));
        }
        if let Some(w) = window_weak_k_up.upgrade() {
            w.global::<SettingsBridge>().set_keybindings(ModelRc::from(Rc::new(VecModel::from(list.clone()))));
        }
    });

    let keys_ref_rst = keys_state.clone();
    let window_weak_k_rst = window.as_weak();
    let notif_k_rst = ctx.notifications.clone();
    bridge.on_reset_keybinding(move |id| {
        let mut list = keys_ref_rst.borrow_mut();
        if let Some(item) = list.iter_mut().find(|it| it.id == id) {
            item.primary_key = item.default_key.clone();
            item.is_customized = false;
            notif_k_rst.info("快捷键已恢复默认", &format!("「{}」已恢复默认键位: {}", item.action_name, item.default_key));
        }
        if let Some(w) = window_weak_k_rst.upgrade() {
            w.global::<SettingsBridge>().set_keybindings(ModelRc::from(Rc::new(VecModel::from(list.clone()))));
        }
    });

    let keys_ref_all = keys_state.clone();
    let window_weak_k_all = window.as_weak();
    let notif_k_all = ctx.notifications.clone();
    bridge.on_reset_all_keybindings(move || {
        let mut list = keys_ref_all.borrow_mut();
        for item in list.iter_mut() {
            item.primary_key = item.default_key.clone();
            item.is_customized = false;
        }
        notif_k_all.info("全部快捷键已重置", "所有终端与全局快捷键已恢复为官方默认映射");
        if let Some(w) = window_weak_k_all.upgrade() {
            w.global::<SettingsBridge>().set_keybindings(ModelRc::from(Rc::new(VecModel::from(list.clone()))));
        }
    });

    let keys_ref_clr = keys_state.clone();
    let window_weak_k_clr = window.as_weak();
    let notif_k_clr = ctx.notifications.clone();
    bridge.on_clear_keybinding(move |id| {
        let mut list = keys_ref_clr.borrow_mut();
        if let Some(item) = list.iter_mut().find(|it| it.id == id) {
            item.primary_key = "".into();
            item.is_customized = true;
            notif_k_clr.warning("快捷键已解除绑定", &format!("「{}」快捷键已清空解绑", item.action_name));
        }
        if let Some(w) = window_weak_k_clr.upgrade() {
            w.global::<SettingsBridge>().set_keybindings(ModelRc::from(Rc::new(VecModel::from(list.clone()))));
        }
    });

    // 快捷键按键捕获与实时格式化回调 (Keybinding Event Formatter)
    bridge.on_format_keybinding_event(move |text, ctrl, alt, shift, meta| {
        let t_str = text.as_str();
        if t_str == "\u{0011}" || t_str == "\u{0012}" || t_str == "\u{0010}" || t_str == "Control" || t_str == "Shift" || t_str == "Alt" || t_str == "Meta" {
            return "".into();
        }

        let main_key = match t_str {
            "\n" | "\r" | "Return" => "Enter",
            "\t" | "Tab" => "Tab",
            "\u{0008}" | "Backspace" => "Backspace",
            "\u{007f}" | "Delete" => "Delete",
            " " => "Space",
            "\u{001b}" | "Escape" => "Esc",
            "UpArrow" | "\u{f700}" => "Up",
            "DownArrow" | "\u{f701}" => "Down",
            "LeftArrow" | "\u{f702}" => "Left",
            "RightArrow" | "\u{f703}" => "Right",
            "F1" => "F1",
            "F2" => "F2",
            "F3" => "F3",
            "F4" => "F4",
            "F5" => "F5",
            "F6" => "F6",
            "F7" => "F7",
            "F8" => "F8",
            "F9" => "F9",
            "F10" => "F10",
            "F11" => "F11",
            "F12" => "F12",
            _ => {
                let trimmed = t_str.trim();
                if trimmed.is_empty() {
                    return "".into();
                }
                trimmed
            }
        };

        if main_key.is_empty() {
            return "".into();
        }

        let mut parts = Vec::new();
        if ctrl { parts.push("Ctrl"); }
        if alt { parts.push("Alt"); }
        if shift { parts.push("Shift"); }
        if meta { parts.push("Meta"); }

        let upper_main = main_key.to_uppercase();
        let display_main = if main_key.len() == 1 {
            upper_main.as_str()
        } else {
            main_key
        };
        parts.push(display_main);

        parts.join("+").into()
    });
}
