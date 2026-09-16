//! 终端高亮规则管理与拾色器交互处理器

use std::cell::RefCell;
use std::rc::Rc;
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::generated::{AppWindow, KeywordHighlightRule, SettingsBridge};
use crate::handlers::AppContext;
use crate::handlers::color_utils::{hsv_to_rgb, rgb_to_hsv, parse_hex_to_rgb};
use super::utils::hex_to_slint_color;

pub(crate) fn register_highlight_handlers(window: &AppWindow, ctx: &AppContext) {
    let bridge = window.global::<SettingsBridge>();

    // -------------------------------------------------------------------------
    // 终端运维高亮规则管理 (Keyword & Regex Highlighting Rules)
    // -------------------------------------------------------------------------
    let initial_rules = vec![
        KeywordHighlightRule {
            id: "kw_err".into(),
            pattern: "\\b(ERROR|FATAL|CRITICAL|Failed|Error)\\b".into(),
            remark: "致命错误与失败".into(),
            color_hex: "#EF4444".into(),
            rule_color: hex_to_slint_color("#EF4444"),
            enabled: true,
        },
        KeywordHighlightRule {
            id: "kw_warn".into(),
            pattern: "\\b(WARN|WARNING|Warning|Warn)\\b".into(),
            remark: "告警提示与注意".into(),
            color_hex: "#F59E0B".into(),
            rule_color: hex_to_slint_color("#F59E0B"),
            enabled: true,
        },
        KeywordHighlightRule {
            id: "kw_ok".into(),
            pattern: "\\b(SUCCESS|OK|Finished|Done)\\b".into(),
            remark: "执行成功与确认".into(),
            color_hex: "#10B981".into(),
            rule_color: hex_to_slint_color("#10B981"),
            enabled: true,
        },
        KeywordHighlightRule {
            id: "kw_url".into(),
            pattern: "https?://[^\\s/$.?#].[^\\s]*".into(),
            remark: "网络超链接 URL".into(),
            color_hex: "#3B82F6".into(),
            rule_color: hex_to_slint_color("#3B82F6"),
            enabled: true,
        },
        KeywordHighlightRule {
            id: "kw_ip".into(),
            pattern: "\\b\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\b".into(),
            remark: "IPv4 主机地址".into(),
            color_hex: "#8B5CF6".into(),
            rule_color: hex_to_slint_color("#8B5CF6"),
            enabled: true,
        },
    ];
    let rules_state = Rc::new(RefCell::new(initial_rules.clone()));
    bridge.set_terminal_keyword_rules(ModelRc::from(Rc::new(VecModel::from(initial_rules.clone()))));
    window.global::<SettingsBridge>().set_terminal_keyword_rules(ModelRc::from(Rc::new(VecModel::from(initial_rules))));

    // 注册添加规则
    let rules_state_add = rules_state.clone();
    let window_weak_add = window.as_weak();
    let notif_add = ctx.notifications.clone();
    bridge.on_add_keyword_rule(move |pattern, remark, color_hex| {
        let p = pattern.as_str().trim();
        if p.is_empty() { return; }
        let mut list = rules_state_add.borrow_mut();
        let new_id = format!("kw_{}", list.len() + 1);
        let rule = KeywordHighlightRule {
            id: new_id.into(),
            pattern: p.into(),
            remark: remark.clone(),
            color_hex: color_hex.clone(),
            rule_color: hex_to_slint_color(color_hex.as_str()),
            enabled: true,
        };
        list.push(rule);
        notif_add.success("已添加高亮规则", &format!("成功添加规则「{}」", p));
        if let Some(w) = window_weak_add.upgrade() {
            let model = ModelRc::from(Rc::new(VecModel::from(list.clone())));
            w.global::<SettingsBridge>().set_terminal_keyword_rules(model.clone());
            w.global::<SettingsBridge>().set_terminal_keyword_rules(model);
        }
    });

    // 注册编辑规则
    let rules_state_edit = rules_state.clone();
    let window_weak_edit = window.as_weak();
    let notif_edit = ctx.notifications.clone();
    bridge.on_update_keyword_rule(move |id, pattern, remark, color_hex| {
        let p = pattern.as_str().trim();
        if p.is_empty() { return; }
        let mut list = rules_state_edit.borrow_mut();
        if let Some(item) = list.iter_mut().find(|r| r.id == id) {
            item.pattern = p.into();
            item.remark = remark.clone();
            item.color_hex = color_hex.clone();
            item.rule_color = hex_to_slint_color(color_hex.as_str());
            notif_edit.success("高亮规则已更新", &format!("成功更新规则「{}」", p));
        }
        if let Some(w) = window_weak_edit.upgrade() {
            let model = ModelRc::from(Rc::new(VecModel::from(list.clone())));
            w.global::<SettingsBridge>().set_terminal_keyword_rules(model.clone());
            w.global::<SettingsBridge>().set_terminal_keyword_rules(model);
        }
    });

    // 注册开关规则
    let rules_state_toggle = rules_state.clone();
    let window_weak_toggle = window.as_weak();
    bridge.on_toggle_keyword_rule(move |id, enabled| {
        let mut list = rules_state_toggle.borrow_mut();
        if let Some(item) = list.iter_mut().find(|r| r.id == id) {
            item.enabled = enabled;
        }
        if let Some(w) = window_weak_toggle.upgrade() {
            let model = ModelRc::from(Rc::new(VecModel::from(list.clone())));
            w.global::<SettingsBridge>().set_terminal_keyword_rules(model.clone());
            w.global::<SettingsBridge>().set_terminal_keyword_rules(model);
        }
    });

    // 注册删除规则
    let rules_state_del = rules_state.clone();
    let window_weak_del = window.as_weak();
    let notif_del = ctx.notifications.clone();
    bridge.on_delete_keyword_rule(move |id| {
        let mut list = rules_state_del.borrow_mut();
        list.retain(|r| r.id != id);
        notif_del.info("规则已移除", "已成功删除该条终端高亮规则");
        if let Some(w) = window_weak_del.upgrade() {
            let model = ModelRc::from(Rc::new(VecModel::from(list.clone())));
            w.global::<SettingsBridge>().set_terminal_keyword_rules(model.clone());
            w.global::<SettingsBridge>().set_terminal_keyword_rules(model);
        }
    });

    // 注册高亮规则拾色器圆盘交互
    let cur_hsv = Rc::new(RefCell::new((217.0f32, 0.77f32, 0.96f32)));

    let window_weak_cw_open = window.as_weak();
    let hsv_open = cur_hsv.clone();
    bridge.on_open_keyword_color_wheel(move || {
        if let Some(w) = window_weak_cw_open.upgrade() {
            let sb = w.global::<SettingsBridge>();
            let hex_str = sb.get_new_rule_color().to_string();
            if let Some((r, g, b)) = parse_hex_to_rgb(&hex_str) {
                let (hue, sat, val) = rgb_to_hsv(r, g, b);
                let mut hsv = hsv_open.borrow_mut();
                hsv.0 = hue;
                hsv.1 = sat;
                hsv.2 = val;

                let angle_rad = hue.to_radians();
                let ind_x = 90.0 + sat * angle_rad.cos() * 88.0;
                let ind_y = 90.0 + sat * angle_rad.sin() * 88.0;
                let (hr, hg, hb) = hsv_to_rgb(hue, 1.0, 1.0);
                let hue_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(hr, hg, hb));
                let cur_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(r, g, b));

                sb.set_color_picker_indicator_x(ind_x);
                sb.set_color_picker_indicator_y(ind_y);
                sb.set_color_picker_brightness(val);
                sb.set_color_picker_hue_brush(hue_brush);
                sb.set_color_picker_preview_brush(cur_brush);
                sb.set_color_picker_hex(hex_str.as_str().into());
            }
            sb.set_is_keyword_color_wheel_open(true);
        }
    });

    let window_weak_cw_coord = window.as_weak();
    let hsv_coord = cur_hsv.clone();
    bridge.on_color_wheel_coord_picked(move |rel_x, rel_y| {
        if let Some(w) = window_weak_cw_coord.upgrade() {
            let dist = (rel_x * rel_x + rel_y * rel_y).sqrt().min(1.0);
            let angle = rel_y.atan2(rel_x).to_degrees();
            let hue = if angle < 0.0 { angle + 360.0 } else { angle };
            let sat = dist;
            let mut hsv = hsv_coord.borrow_mut();
            hsv.0 = hue;
            hsv.1 = sat;
            let val = hsv.2;
            let (r, g, b) = hsv_to_rgb(hue, sat, val);
            let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
            let (hr, hg, hb) = hsv_to_rgb(hue, 1.0, 1.0);
            let hue_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(hr, hg, hb));
            let cur_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(r, g, b));

            let norm_dx = if dist > 0.0 { rel_x / dist * dist.min(1.0) } else { 0.0 };
            let norm_dy = if dist > 0.0 { rel_y / dist * dist.min(1.0) } else { 0.0 };
            let sb = w.global::<SettingsBridge>();
            sb.set_color_picker_indicator_x(90.0 + norm_dx * 88.0);
            sb.set_color_picker_indicator_y(90.0 + norm_dy * 88.0);
            sb.set_color_picker_hex(hex.as_str().into());
            sb.set_color_picker_preview_brush(cur_brush);
            sb.set_color_picker_hue_brush(hue_brush);
        }
    });

    let window_weak_cw_bright = window.as_weak();
    let hsv_bright = cur_hsv.clone();
    bridge.on_color_wheel_brightness_picked(move |brightness| {
        if let Some(w) = window_weak_cw_bright.upgrade() {
            let val = brightness.clamp(0.0, 1.0);
            let mut hsv = hsv_bright.borrow_mut();
            hsv.2 = val;
            let (r, g, b) = hsv_to_rgb(hsv.0, hsv.1, val);
            let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
            let cur_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(r, g, b));

            let sb = w.global::<SettingsBridge>();
            sb.set_color_picker_brightness(val);
            sb.set_color_picker_hex(hex.as_str().into());
            sb.set_color_picker_preview_brush(cur_brush);
        }
    });

    let window_weak_cw_hex = window.as_weak();
    let hsv_hex = cur_hsv.clone();
    bridge.on_color_picker_hex_changed(move |hex_val| {
        if let Some(w) = window_weak_cw_hex.upgrade() {
            let hex_str = hex_val.to_string();
            if let Some((r, g, b)) = parse_hex_to_rgb(&hex_str) {
                let (h_deg, s, v) = rgb_to_hsv(r, g, b);
                let mut hsv = hsv_hex.borrow_mut();
                hsv.0 = h_deg;
                hsv.1 = s;
                hsv.2 = v;

                let angle_rad = h_deg.to_radians();
                let ind_x = 90.0 + s * angle_rad.cos() * 88.0;
                let ind_y = 90.0 + s * angle_rad.sin() * 88.0;
                let (hr, hg, hb) = hsv_to_rgb(h_deg, 1.0, 1.0);
                let hue_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(hr, hg, hb));
                let cur_brush = slint::Brush::SolidColor(slint::Color::from_rgb_u8(r, g, b));

                let sb = w.global::<SettingsBridge>();
                sb.set_color_picker_indicator_x(ind_x);
                sb.set_color_picker_indicator_y(ind_y);
                sb.set_color_picker_brightness(v);
                sb.set_color_picker_hue_brush(hue_brush);
                sb.set_color_picker_preview_brush(cur_brush);
                sb.set_color_picker_hex(hex_str.as_str().into());
            }
        }
    });
}
