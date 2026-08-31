//! 历史会话管理与 Slint UI 抽屉视图数据同步。

use std::rc::Rc;
use slint::ComponentHandle;
use smagical_core::HistoryRecord;


use crate::generated::{AppWindow, HistoryGroupData, HistoryItemData};
use crate::handlers::AppContext;


/// 将 Unix 秒时间戳格式化为本地可读的具体日期时间字符串 (格式: "YYYY-MM-DD HH:MM:SS")
pub(crate) fn format_datetime(timestamp: u64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }
    let total_secs = timestamp + 8 * 3600; // 默认采用 CST (UTC+8) 显示
    let secs_of_day = total_secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    let mut days = (total_secs / 86400) as i64;
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, hours, minutes, seconds)
}

/// 将 Unix 秒时间戳格式化为具体时间字符串 (格式: "HH:MM:SS")
pub(crate) fn format_time_only(timestamp: u64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }
    let total_secs = timestamp + 8 * 3600; // 默认采用 CST (UTC+8) 显示
    let secs_of_day = total_secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// 格式化持续时长文本 (如 "42m", "1h 15m", "2s")
pub(crate) fn format_duration(secs: u64) -> String {

    if secs == 0 {
        "1s".to_string()
    } else if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 { format!("{}m", m) } else { format!("{}m {}s", m, s) }
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m == 0 { format!("{}h", h) } else { format!("{}h {}m", h, m) }
    }
}

/// 将单个历史记录实体转换为 Slint UI 数据项
fn map_history_item(r: &HistoryRecord, _now: u64, is_aggregated: bool) -> HistoryItemData {
    let conn_dt = format_datetime(r.connected_at);
    let disc_time = if let Some(disc) = r.disconnected_at {
        format_time_only(disc)
    } else if r.exit_status == "active" {
        "进行中".to_string()
    } else {
        format_time_only(r.connected_at + r.duration_secs)
    };
    let dur_text = format_duration(r.duration_secs);
    let status_desc = match r.exit_status.as_str() {
        "active" => "🟢 活跃中",
        "success" | "closed" => "⚪ 正常退出",
        "timeout" => "🔴 连接超时",
        "auth_failed" => "🟠 认证失败",
        "error" => "🔴 异常中断",
        _ => "⚪ 正常退出",
    };

    let subtitle = if is_aggregated {
        format!("{}@{} · 累计连接 {} 次 · 最近连接: {} · 累计时长: {}", r.username, r.address, r.connect_count, conn_dt, dur_text)
    } else {
        format!("{}@{} · 连接: {} · 断开: {} · 统计时长: {} · {}", r.username, r.address, conn_dt, disc_time, dur_text, status_desc)
    };

    let time_text = conn_dt.clone();

    HistoryItemData {
        id: r.id.clone().into(),
        host_id: r.host_id.clone().unwrap_or_default().into(),
        title: r.title.clone().into(),
        address: r.address.clone().into(),
        username: r.username.clone().into(),
        session_type: r.session_type.clone().into(),
        time_text: time_text.into(),
        duration_text: dur_text.into(),
        exit_status: r.exit_status.clone().into(),
        error_msg: r.error_msg.clone().unwrap_or_default().into(),
        is_pinned: r.is_pinned,
        connect_count: r.connect_count as i32,
        subtitle: subtitle.into(),
    }
}


/// 独立更新 Slint 历史抽屉数据与视图 (线程安全入参)
pub(crate) fn sync_ui_history_from_state(
    window: &AppWindow,
    storage: &dyn smagical_core::AppStorage,
    search_q: &str,
    view_mode: &str,
    collapsed_set: &std::collections::HashSet<String>,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1725019200);

    let all_records = storage.history().list_all().unwrap_or_default();
    let total_count = all_records.len() as i32;
    let search_q_lower = search_q.to_lowercase();

    // 过滤逻辑
    let filtered_records: Vec<HistoryRecord> = if search_q_lower.is_empty() {
        all_records
    } else {
        all_records
            .into_iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&search_q_lower)
                    || r.address.to_lowercase().contains(&search_q_lower)
                    || r.username.to_lowercase().contains(&search_q_lower)
                    || r.session_type.to_lowercase().contains(&search_q_lower)
            })
            .collect()
    };

    let groups: Vec<HistoryGroupData> = if view_mode == "hosts" {
        // 按主机聚合模式 (去重并累加连接次数)
        let mut host_map: std::collections::HashMap<String, HistoryRecord> = std::collections::HashMap::new();
        for r in &filtered_records {
            let key = if let Some(ref hid) = r.host_id {
                hid.clone()
            } else {
                r.address.clone()
            };
            host_map
                .entry(key)
                .and_modify(|existing| {
                    existing.connect_count += r.connect_count.max(1);
                    if r.connected_at > existing.connected_at {
                        existing.connected_at = r.connected_at;
                        existing.exit_status = r.exit_status.clone();
                        existing.duration_secs = r.duration_secs;
                    }
                    if r.is_pinned {
                        existing.is_pinned = true;
                    }
                })
                .or_insert_with(|| r.clone());
        }

        let mut aggregated_list: Vec<HistoryRecord> = host_map.into_values().collect();
        aggregated_list.sort_by(|a, b| {
            b.is_pinned.cmp(&a.is_pinned)
                .then_with(|| b.connect_count.cmp(&a.connect_count))
                .then_with(|| b.connected_at.cmp(&a.connected_at))
        });

        let items: Vec<HistoryItemData> = aggregated_list
            .iter()
            .map(|r| map_history_item(r, now, true))
            .collect();

        if items.is_empty() {
            Vec::new()
        } else {
            vec![HistoryGroupData {
                group_id: "all_hosts".into(),
                group_name: "全部主机 (按频次)".into(),
                item_count: items.len() as i32,
                is_collapsed: collapsed_set.contains("all_hosts"),
                items: slint::ModelRc::from(Rc::new(slint::VecModel::from(items))),
            }]
        }
    } else {
        // 时间流模式 (Timeline)
        let mut pinned_items = Vec::new();
        let mut today_items = Vec::new();
        let mut yesterday_items = Vec::new();
        let mut earlier_items = Vec::new();

        for r in &filtered_records {
            let item = map_history_item(r, now, false);
            if r.is_pinned {
                pinned_items.push(item);
            } else {
                let diff = now.saturating_sub(r.connected_at);
                if diff < 86400 {
                    today_items.push(item);
                } else if diff < 172800 {
                    yesterday_items.push(item);
                } else {
                    earlier_items.push(item);
                }
            }
        }

        let mut result_groups = Vec::new();
        if !pinned_items.is_empty() {
            result_groups.push(HistoryGroupData {
                group_id: "pinned".into(),
                group_name: "置顶常用".into(),
                item_count: pinned_items.len() as i32,
                is_collapsed: collapsed_set.contains("pinned"),
                items: slint::ModelRc::from(Rc::new(slint::VecModel::from(pinned_items))),
            });
        }
        if !today_items.is_empty() {
            result_groups.push(HistoryGroupData {
                group_id: "today".into(),
                group_name: "今天".into(),
                item_count: today_items.len() as i32,
                is_collapsed: collapsed_set.contains("today"),
                items: slint::ModelRc::from(Rc::new(slint::VecModel::from(today_items))),
            });
        }
        if !yesterday_items.is_empty() {
            result_groups.push(HistoryGroupData {
                group_id: "yesterday".into(),
                group_name: "昨天".into(),
                item_count: yesterday_items.len() as i32,
                is_collapsed: collapsed_set.contains("yesterday"),
                items: slint::ModelRc::from(Rc::new(slint::VecModel::from(yesterday_items))),
            });
        }
        if !earlier_items.is_empty() {
            result_groups.push(HistoryGroupData {
                group_id: "earlier".into(),
                group_name: "更早".into(),
                item_count: earlier_items.len() as i32,
                is_collapsed: collapsed_set.contains("earlier"),
                items: slint::ModelRc::from(Rc::new(slint::VecModel::from(earlier_items))),
            });
        }

        result_groups
    };

    window.set_history_total_count(total_count);
    window.set_history_groups(slint::ModelRc::from(Rc::new(slint::VecModel::from(groups))));
}

/// 同步更新 Slint 历史抽屉数据与视图
pub(crate) fn sync_ui_history(window: &AppWindow, ctx: &AppContext) {
    let search_q = ctx.history_search_query.borrow().clone();
    let view_mode = ctx.history_view_mode.borrow().clone();
    let collapsed_set = ctx.collapsed_history_groups.borrow().clone();
    sync_ui_history_from_state(
        window,
        ctx.core_state.storage().as_ref(),
        &search_q,
        &view_mode,
        &collapsed_set,
    );
}


/// 注册历史会话抽屉交互回调
pub(crate) fn register_history_handlers(window: &AppWindow, ctx: &AppContext) {
    // 1. 重连历史会话 (在当前焦点窗格中打开)
    let window_weak = window.as_weak();
    let ctx_recon = ctx.clone();
    window.on_reconnect_history(move |hist_id| {
        if let Some(w) = window_weak.upgrade() {
            let hist_opt = ctx_recon.core_state.storage().history().get_by_id(&hist_id).unwrap_or_default();
            if let Some(mut h) = hist_opt {
                let target_id = h.host_id.clone().unwrap_or_else(|| h.id.clone());

                // 更新历史连接频次与时间
                h.connected_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(1725019200);
                h.exit_status = "active".to_string();
                h.connect_count += 1;
                let _ = ctx_recon.core_state.storage().history().save(&h);

                tracing::info!(target: "smagical_ui::history", "历史会话触发重连: {} ({})", h.title, target_id);
                sync_ui_history(&w, &ctx_recon);

                // 发起连接
                w.invoke_open_host(target_id.into());
            }
        }
    });

    // 2. 重连历史会话并在右侧垂直分屏打开
    let window_weak = window.as_weak();
    let ctx_recon_split = ctx.clone();
    window.on_reconnect_history_split(move |hist_id| {
        if let Some(w) = window_weak.upgrade() {
            let hist_opt = ctx_recon_split.core_state.storage().history().get_by_id(&hist_id).unwrap_or_default();
            if let Some(mut h) = hist_opt {
                let target_id = h.host_id.clone().unwrap_or_else(|| h.id.clone());

                // 先执行垂直分屏
                w.invoke_split_terminal("vertical".into());

                // 更新历史记录
                h.connected_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(1725019200);
                h.exit_status = "active".to_string();
                h.connect_count += 1;
                let _ = ctx_recon_split.core_state.storage().history().save(&h);

                tracing::info!(target: "smagical_ui::history", "历史会话分屏重连: {} ({})", h.title, target_id);
                sync_ui_history(&w, &ctx_recon_split);

                // 在新分屏中打开
                w.invoke_open_host(target_id.into());
            }
        }
    });

    // 3. 删除单条历史记录
    let window_weak = window.as_weak();
    let ctx_del = ctx.clone();
    window.on_delete_history_item(move |hist_id| {
        if let Some(w) = window_weak.upgrade() {
            let _ = ctx_del.core_state.storage().history().delete(&hist_id);
            tracing::info!(target: "smagical_ui::history", "删除历史会话记录: {}", hist_id);
            sync_ui_history(&w, &ctx_del);
        }
    });

    // 4. 清空全部历史记录 (保留置顶项)
    let window_weak = window.as_weak();
    let ctx_clr = ctx.clone();
    window.on_clear_history(move || {
        if let Some(w) = window_weak.upgrade() {
            let _ = ctx_clr.core_state.storage().history().clear_all(true);
            tracing::info!(target: "smagical_ui::history", "清空历史记录 (保留置顶项)");
            sync_ui_history(&w, &ctx_clr);
        }
    });

    // 5. 切换单条历史置顶标星
    let window_weak = window.as_weak();
    let ctx_pin = ctx.clone();
    window.on_toggle_pin_history(move |hist_id| {
        if let Some(w) = window_weak.upgrade() {
            let is_pinned = ctx_pin.core_state.storage().history().toggle_pin(&hist_id).unwrap_or_default();
            tracing::info!(target: "smagical_ui::history", "切换历史会话置顶状态: {} -> {}", hist_id, if is_pinned { "⭐️ 已置顶" } else { "取消置顶" });
            sync_ui_history(&w, &ctx_pin);
        }
    });


    // 6. 切换时间分组折叠展开
    let window_weak = window.as_weak();
    let ctx_grp = ctx.clone();
    window.on_toggle_history_group(move |group_id| {
        if let Some(w) = window_weak.upgrade() {
            let mut set = ctx_grp.collapsed_history_groups.borrow_mut();
            let gid = group_id.to_string();
            let is_collapsed = if set.contains(&gid) {
                set.remove(&gid);
                false
            } else {
                set.insert(gid.clone());
                true
            };
            drop(set);
            tracing::debug!(target: "smagical_ui::history", "切换历史时间分组折叠: {} ({})", gid, if is_collapsed { "已折叠" } else { "已展开" });
            sync_ui_history(&w, &ctx_grp);
        }
    });

    // 7. 历史记录实时搜索过滤
    let window_weak = window.as_weak();
    let ctx_search = ctx.clone();
    window.on_filter_history(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let q = query.trim().to_string();
            *ctx_search.history_search_query.borrow_mut() = q.clone();
            if !q.is_empty() {
                tracing::debug!(target: "smagical_ui::history", "历史记录搜索过滤: \"{}\"", q);
            }
            sync_ui_history(&w, &ctx_search);
        }
    });

    // 8. 切换历史抽屉视图模式 (时间流 / 按主机)
    let window_weak = window.as_weak();
    let ctx_mode = ctx.clone();
    window.on_switch_history_view_mode(move |mode| {
        if let Some(w) = window_weak.upgrade() {
            *ctx_mode.history_view_mode.borrow_mut() = mode.to_string();
            tracing::debug!(target: "smagical_ui::history", "切换历史视图模式: {}", mode);
            sync_ui_history(&w, &ctx_mode);
        }
    });

    // 9. 打开历史详情与终端快照弹窗
    let window_weak = window.as_weak();
    let ctx_detail = ctx.clone();
    window.on_show_history_detail(move |hist_id| {
        if let Some(w) = window_weak.upgrade()
            && let Ok(Some(hist)) = ctx_detail.core_state.storage().history().get_by_id(&hist_id)
        {
            let snapshot = ctx_detail.core_state.storage().history().get_snapshot(&hist_id).unwrap_or_default().unwrap_or_default();
            let snapshot_lines = snapshot.lines().count() as i32;
            let conn_dt = format_datetime(hist.connected_at);
            let disc_dt = if let Some(disc) = hist.disconnected_at {
                format_datetime(disc)
            } else if hist.exit_status == "active" {
                String::new()
            } else {
                format_datetime(hist.connected_at + hist.duration_secs)
            };
            let dur = format_duration(hist.duration_secs);

            w.set_history_detail_id(hist.id.clone().into());
            w.set_history_detail_title(hist.title.clone().into());
            w.set_history_detail_address(format!("{}:{}", hist.address, hist.port).into());
            w.set_history_detail_user(hist.username.clone().into());
            w.set_history_detail_type(hist.session_type.into());
            w.set_history_detail_connected_time(conn_dt.into());
            w.set_history_detail_disconnected_time(disc_dt.into());
            w.set_history_detail_duration(dur.into());
            w.set_history_detail_exit_status(hist.exit_status.into());
            w.set_history_detail_error_msg(hist.error_msg.unwrap_or_default().into());
            w.set_history_detail_snapshot(snapshot.into());
            w.set_history_detail_snapshot_lines(snapshot_lines);
            w.set_is_history_detail_open(true);

            tracing::info!(target: "smagical_ui::history", "查看历史会话详情与快照: {} (用户: {}, 地址: {}, 快照: {} 行)", hist.title, hist.username, hist.address, snapshot_lines);
        }
    });

    // 10. 复制历史终端快照日志到剪贴板
    window.on_copy_history_log(move |content| {
        let chars = content.chars().count();
        let lines = content.lines().count();
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(content.to_string());
            tracing::info!(target: "smagical_ui::history", "终端输出快照已复制到系统剪贴板 (共 {} 行, {} 字符)", lines, chars);
        }
    });

    // 11. 活动栏视图切换导航日志与全局 Hook 广播
    let core_state_nav = ctx.core_state.clone();
    window.on_activity_tab_switched(move |tab_id| {
        core_state_nav.app_hooks().dispatch_left_menu_clicked(&tab_id, "");
        tracing::info!(target: "smagical_ui::navigation", "导航切换侧边栏/主页面视图: [{}]", tab_id);
    });



}


