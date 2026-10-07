//! 终端高亮规则管理与拾色器交互处理器

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use slint::{ComponentHandle, Model};
use smagical_core::AppStorage;
use smagical_core::domain::config::KeywordHighlightRuleRecord;
use crate::async_util::spawn_async;
use crate::common::{run_on_ui, to_model_rc};

use crate::generated::{AppWindow, KeywordHighlightRule, SettingsBridge};
use crate::handlers::AppContext;
use crate::handlers::color_utils::{hsv_to_rgb, rgb_to_hsv, parse_hex_to_rgb};
use super::utils::hex_to_slint_color;

/// 将 Slint UI 前端高亮规则视图模型转换为持久化实体记录
///
/// # 参数
/// - `list`: Slint UI 层传入的关键词高亮规则数组切片；
///
/// # 返回值
/// 返回准备写入 SQLite 仓储配置表的 `Vec<KeywordHighlightRuleRecord>` 记录集。
pub(crate) fn rules_to_records(list: &[KeywordHighlightRule]) -> Vec<KeywordHighlightRuleRecord> {
    list.iter().map(|r| KeywordHighlightRuleRecord {
        id: r.id.to_string(),
        pattern: r.pattern.to_string(),
        remark: r.remark.to_string(),
        color_hex: r.color_hex.to_string(),
        enabled: r.enabled,
    }).collect()
}

/// 异步将终端关键词高亮规则持久化保存至应用配置仓储
///
/// # 参数
/// - `storage`: 全局仓储服务抽象接口 `Arc<dyn AppStorage>`；
/// - `records`: 待落盘的规则实体记录集合。
pub(crate) fn persist_keyword_rules(storage: &Arc<dyn AppStorage>, records: Vec<KeywordHighlightRuleRecord>) {
    let storage = storage.clone();
    spawn_async(async move {
        let _ = storage.config().update(Box::new(move |c| {
            c.keyword_highlight_rules = records;
        })).await;
    });
}

/// 注册终端运维高亮规则管理与 HSV 拾色器相关 UI 回调
///
/// 包含以下功能模块：
/// 1. **默认与既有规则加载**：内置 ERROR/WARN/SUCCESS/URL/IPv4 常用运维规则模板；
/// 2. **规则增删改查交互**：新增、编辑、开关切换、物理删除规则，并在修改后自动同步至活跃终端渲染器；
/// 3. **极坐标 HSV 拾色器圆盘交互**：
///    - `on_open_keyword_color_wheel`: 打开拾色器并根据 Hex 解析对应极坐标角度与饱和度半径；
///    - `on_color_wheel_coord_picked`: 拾色器圆盘拖拽坐标换算为色相 (0~360°) 与饱和度 (0~1)；
///    - `on_color_wheel_brightness_picked`: 拾色器明度滑块调节；
///    - `on_color_picker_hex_changed`: 直接手动键入十六进制颜色字符串进行实时反解。
///
/// # 参数
/// - `window`: Slint 顶级应用窗口；
/// - `ctx`: 全局应用上下文。
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
    let existing = bridge.get_terminal_keyword_rules();
    let current_rules: Vec<KeywordHighlightRule> = if existing.row_count() > 0 {
        existing.iter().collect()
    } else {
        initial_rules
    };
    let rules_state = Rc::new(RefCell::new(current_rules.clone()));
    sync_rules_to_renderer(ctx, &current_rules);
    bridge.set_terminal_keyword_rules(to_model_rc(current_rules));

    // 注册添加规则
    let rules_state_add = rules_state.clone();
    let window_weak_add = window.as_weak();
    let notif_add = ctx.notifications.clone();
    let ctx_add = ctx.clone();
    let storage_add = ctx.core_state.storage().clone();
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
        sync_rules_to_renderer(&ctx_add, &list);
        notif_add.success("已添加高亮规则", &format!("成功添加规则「{}」", p));
        let list_snapshot = list.clone();
        run_on_ui(window_weak_add.clone(), move |w| {
            w.global::<SettingsBridge>().set_terminal_keyword_rules(to_model_rc(list_snapshot));
        });
        let records = rules_to_records(&list);
        persist_keyword_rules(&storage_add, records);
    });

    // 注册编辑规则
    let rules_state_edit = rules_state.clone();
    let window_weak_edit = window.as_weak();
    let notif_edit = ctx.notifications.clone();
    let ctx_edit = ctx.clone();
    let storage_edit = ctx.core_state.storage().clone();
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
        sync_rules_to_renderer(&ctx_edit, &list);
        let list_snapshot = list.clone();
        run_on_ui(window_weak_edit.clone(), move |w| {
            w.global::<SettingsBridge>().set_terminal_keyword_rules(to_model_rc(list_snapshot));
        });
        let records = rules_to_records(&list);
        persist_keyword_rules(&storage_edit, records);
    });

    // 注册开关规则
    let rules_state_toggle = rules_state.clone();
    let window_weak_toggle = window.as_weak();
    let ctx_toggle = ctx.clone();
    let storage_toggle = ctx.core_state.storage().clone();
    bridge.on_toggle_keyword_rule(move |id, enabled| {
        let mut list = rules_state_toggle.borrow_mut();
        if let Some(item) = list.iter_mut().find(|r| r.id == id) {
            item.enabled = enabled;
        }
        sync_rules_to_renderer(&ctx_toggle, &list);
        let list_snapshot = list.clone();
        run_on_ui(window_weak_toggle.clone(), move |w| {
            w.global::<SettingsBridge>().set_terminal_keyword_rules(to_model_rc(list_snapshot));
        });
        let records = rules_to_records(&list);
        persist_keyword_rules(&storage_toggle, records);
    });

    // 注册删除规则
    let rules_state_del = rules_state.clone();
    let window_weak_del = window.as_weak();
    let notif_del = ctx.notifications.clone();
    let ctx_del = ctx.clone();
    let storage_del = ctx.core_state.storage().clone();
    bridge.on_delete_keyword_rule(move |id| {
        let mut list = rules_state_del.borrow_mut();
        list.retain(|r| r.id != id);
        sync_rules_to_renderer(&ctx_del, &list);
        notif_del.info("规则已移除", "已成功删除该条终端高亮规则");
        let list_snapshot = list.clone();
        run_on_ui(window_weak_del.clone(), move |w| {
            w.global::<SettingsBridge>().set_terminal_keyword_rules(to_model_rc(list_snapshot));
        });
        let records = rules_to_records(&list);
        persist_keyword_rules(&storage_del, records);
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

/// 将终端运维高亮规则全量下发至活动终端渲染器
///
/// # 算法与数据流
/// 1. **色彩空间转换**：
///    - 遍历所有规则，调用 `parse_hex_to_rgb` 将十六进制颜色字符串（如 `#EF4444`）解析为 RGB 三通道，并附带 Alpha=255 构成 `[u8; 4]`；
///    - 若颜色格式异常，使用防御性回退颜色 `[0xef, 0x44, 0x44, 255]`；
/// 2. **元组结构映射**：
///    - 组装为渲染管线专用元组 `(id, pattern, rgba, enabled)`；
/// 3. **实时热更新**：
///    - 获取活跃终端着色器 `ctx.terminal_renderer` 实例，调用 `update_highlight_rules` 刷新底层正则表达式匹配引擎，使下一次重绘时高亮立即生效。
///
/// # 参数
/// - `ctx`: 应用程序全局上下文句柄；
/// - `list`: 当前界面上的全部关键词高亮规则切片。
fn sync_rules_to_renderer(ctx: &AppContext, list: &[KeywordHighlightRule]) {
    let tuples: Vec<_> = list.iter().map(|r| {
        let rgba = parse_hex_to_rgb(r.color_hex.as_str())
            .map(|(red, g, b)| [red, g, b, 255])
            .unwrap_or([0xef, 0x44, 0x44, 255]);
        (r.id.to_string(), r.pattern.to_string(), rgba, r.enabled)
    }).collect();

    if let Some(ref mut renderer) = *ctx.terminal_renderer.borrow_mut() {
        renderer.update_highlight_rules(tuples);
    }
}
