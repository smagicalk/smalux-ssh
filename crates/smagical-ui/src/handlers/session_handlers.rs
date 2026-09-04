//! 终端多会话管理、Tab 页签操作与快速启动器检索回调绑定。
//!
//! 负责响应前端终端 Tab 切换/关闭、分屏多窗格多 Tab 调度、新建会话中心搜索、终端按键输入、滚轮滚动与剪贴板等交互事件。

use std::rc::Rc;
use slint::ComponentHandle;
use smagical_core::event::{TerminalSessionEvent, TerminalSplitChangedEvent};

use crate::generated::{AppWindow, HostItemData, LocalShellItemData};
use crate::handlers::AppContext;
use crate::session::{sync_active_session_ui, PaneGroup};
use crate::terminal::{encode_key_event, SplitNode, SplitOrientation, TerminalInstance};


/// 核心执行关闭指定终端会话逻辑 (移除 Tab、销毁 PTY、重组分屏并写盘归档)
fn execute_close_session(
    id_str: &str,
    pane_groups: &Rc<std::cell::RefCell<Vec<PaneGroup>>>,
    active_pane_id: &Rc<std::cell::RefCell<String>>,
    global_split_tree: &Rc<std::cell::RefCell<Option<SplitNode>>>,
    active_terminals: &Rc<std::cell::RefCell<std::collections::HashMap<String, TerminalInstance>>>,
    ctx: &AppContext,
    window: &AppWindow,
) {
    let mut groups = pane_groups.borrow_mut();
    let mut active_pid = active_pane_id.borrow_mut();
    let mut split_tree = global_split_tree.borrow_mut();

    // 捕获终端屏幕快照并杀灭底层 PTY 进程与 Alacritty 实例
    let mut snapshot_opt = None;
    if let Some(mut instance) = active_terminals.borrow_mut().remove(id_str) {
        let snap = instance.snapshot_text(500);
        if !snap.trim().is_empty() {
            snapshot_opt = Some((snap.lines().count() as u32, snap));
        }
        let _ = instance.pty.kill();
    }

    let mut target_group_idx = None;
    for (idx, g) in groups.iter_mut().enumerate() {
        if let Some(pos) = g.tabs.iter().position(|t| t.session_id == id_str) {
            g.tabs.remove(pos);
            target_group_idx = Some(idx);
            if g.active_tab_id == id_str && !g.tabs.is_empty() {
                let next_pos = if pos > 0 { pos - 1 } else { 0 };
                g.active_tab_id = g.tabs[next_pos.min(g.tabs.len() - 1)].session_id.clone();
            }
            break;
        }
    }

    if let Some(idx) = target_group_idx
        && groups[idx].tabs.is_empty()
    {
        let closed_pid = groups[idx].pane_id.clone();
        groups.remove(idx);

        if let Some(tree) = split_tree.as_mut() {
            tree.close_pane(&closed_pid);
            if tree.leaf_count() <= 1 {
                *split_tree = None;
            }
        }

        if !groups.is_empty() {
            *active_pid = groups[0].pane_id.clone();
        } else {
            *active_pid = String::new();
        }
    }

    let is_split = split_tree.is_some();
    sync_active_session_ui(window, &groups, &active_pid, is_split);
    crate::session::sync_active_session_to_core(&groups, &active_pid, &ctx.core_state);
    ctx.core_state.events().dispatch(&TerminalSessionEvent {
        session_id: id_str.to_string(),
        host_id: "".into(),
        action: "closed".into(),
    });
    tracing::info!(target: "smagical_ui::session", "已关闭终端会话: {}", id_str);

    let storage_async = ctx.core_state.storage().clone();
    let window_weak_async = window.as_weak();
    let search_q = ctx.history_search_query.borrow().clone();
    let view_mode = ctx.history_view_mode.borrow().clone();
    let collapsed = ctx.collapsed_history_groups.borrow().clone();
    let id_owned = id_str.to_string();

    ctx.persistence_guard.spawn(move || {
        if let Some((lines_count, snap_text)) = snapshot_opt {
            let hist_id = format!("hist-{}", id_owned);
            let _ = storage_async.history().save_snapshot(&hist_id, &snap_text, 500);
            if let Ok(Some(mut h)) = storage_async.history().get_by_id(&hist_id) {
                h.record_snapshot(lines_count);
                let _ = storage_async.history().save(&h);
            }
        }
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w_ui) = window_weak_async.upgrade() {
                crate::handlers::history_handlers::sync_ui_history_from_state(
                    &w_ui,
                    storage_async.as_ref(),
                    &search_q,
                    &view_mode,
                    &collapsed,
                );
            }
        });
    });
}

/// 注册终端会话与启动器相关交互回调。
///
/// 将前端所有终端事件总线（包含多会话 Tab 调度、分屏多 Tab 迁移、快速新建弹窗、按键转发、剪贴板交互等）与底层运行时打通。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用
/// - `ctx`: 全局应用共享上下文对象引用
pub(crate) fn register_session_handlers(window: &AppWindow, ctx: &AppContext) {
    // 待确认关闭的会话 ID 挂起槽位
    let pending_close_tab_id = Rc::new(std::cell::RefCell::new(Option::<String>::None));

    // -------------------------------------------------------------------------
    // 1. 关闭会话 Tab 回调 (带活跃连接前置监听与防误触拦截)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_close = Rc::clone(&ctx.pane_groups);
    let active_pane_id_close = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_close = Rc::clone(&ctx.global_split_tree);
    let active_terminals_close = Rc::clone(&ctx.active_terminals);
    let pending_close_tab_id_close = Rc::clone(&pending_close_tab_id);
    let ctx_close = ctx.clone();
    window.on_close_tab(move |sess_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = sess_id.to_string();

            // 拦截检查：查询目标 Tab 元数据
            let session_opt = pane_groups_close.borrow().iter()
                .flat_map(|g| g.tabs.iter())
                .find(|t| t.session_id == id_str)
                .cloned();

            if w.get_setting_confirm_close_tab() {
                if let Some(info) = session_opt {
                    let is_remote = !info.host_id.starts_with("local-") && !info.host_address.starts_with("Local");
                    let title = if is_remote {
                        format!("断开远程主机连接: {}", info.display_title)
                    } else {
                        format!("关闭终端会话: {}", info.display_title)
                    };
                    let msg = if is_remote {
                        format!("确定要断开与主机 [{}] 的 SSH 连接吗？未保存的工作和远程正在运行的任务将立即终止。", info.host_name)
                    } else {
                        format!("确定要关闭本地终端 [{}] 吗？正在运行的进程将被终止。", info.display_title)
                    };

                    *pending_close_tab_id_close.borrow_mut() = Some(id_str);
                    w.set_tab_close_confirm_title(title.into());
                    w.set_tab_close_confirm_message(msg.into());
                    w.set_is_tab_close_confirm_open(true);
                    tracing::info!(target: "smagical_ui::session", "拦截关闭 Tab 操作并呼出确认弹窗: {}", info.display_title);
                    return;
                }
            }

            execute_close_session(
                &id_str,
                &pane_groups_close,
                &active_pane_id_close,
                &global_split_tree_close,
                &active_terminals_close,
                &ctx_close,
                &w,
            );
        }
    });

    // -------------------------------------------------------------------------
    // 1.1 关闭指定窗格内的指定 Tab 回调
    // -------------------------------------------------------------------------
    let window_weak_pane = window.as_weak();
    let pane_groups_pane = Rc::clone(&ctx.pane_groups);
    let active_pane_id_pane = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_pane = Rc::clone(&ctx.global_split_tree);
    let active_terminals_pane = Rc::clone(&ctx.active_terminals);
    let pending_close_tab_id_pane = Rc::clone(&pending_close_tab_id);
    let ctx_pane = ctx.clone();
    window.on_close_pane_tab(move |_pane_id, tab_id| {
        if let Some(w) = window_weak_pane.upgrade() {
            let id_str = tab_id.to_string();

            let session_opt = pane_groups_pane.borrow().iter()
                .flat_map(|g| g.tabs.iter())
                .find(|t| t.session_id == id_str)
                .cloned();

            if w.get_setting_confirm_close_tab() {
                if let Some(info) = session_opt {
                    let is_remote = !info.host_id.starts_with("local-") && !info.host_address.starts_with("Local");
                    let title = if is_remote {
                        format!("断开远程主机连接: {}", info.display_title)
                    } else {
                        format!("关闭终端会话: {}", info.display_title)
                    };
                    let msg = if is_remote {
                        format!("确定要断开与主机 [{}] 的 SSH 连接吗？未保存的工作和远程正在运行的任务将立即终止。", info.host_name)
                    } else {
                        format!("确定要关闭本地终端 [{}] 吗？正在运行的进程将被终止。", info.display_title)
                    };

                    *pending_close_tab_id_pane.borrow_mut() = Some(id_str);
                    w.set_tab_close_confirm_title(title.into());
                    w.set_tab_close_confirm_message(msg.into());
                    w.set_is_tab_close_confirm_open(true);
                    return;
                }
            }

            execute_close_session(
                &id_str,
                &pane_groups_pane,
                &active_pane_id_pane,
                &global_split_tree_pane,
                &active_terminals_pane,
                &ctx_pane,
                &w,
            );
        }
    });

    // -------------------------------------------------------------------------
    // 1.2 确认关闭单个 Tab 回调 (在确认弹窗中点击“断开并关闭”)
    // -------------------------------------------------------------------------
    let window_weak_confirm = window.as_weak();
    let pane_groups_confirm = Rc::clone(&ctx.pane_groups);
    let active_pane_id_confirm = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_confirm = Rc::clone(&ctx.global_split_tree);
    let active_terminals_confirm = Rc::clone(&ctx.active_terminals);
    let pending_close_tab_id_confirm = Rc::clone(&pending_close_tab_id);
    let ctx_confirm = ctx.clone();
    window.on_confirm_close_tab(move || {
        if let Some(w) = window_weak_confirm.upgrade() {
            if let Some(id_str) = pending_close_tab_id_confirm.borrow_mut().take() {
                execute_close_session(
                    &id_str,
                    &pane_groups_confirm,
                    &active_pane_id_confirm,
                    &global_split_tree_confirm,
                    &active_terminals_confirm,
                    &ctx_confirm,
                    &w,
                );
            }
        }
    });

    // -------------------------------------------------------------------------
    // 1.3 切换“关闭标签页时防误触确认”开关回调
    // -------------------------------------------------------------------------
    let window_weak_toggle = window.as_weak();
    let core_state_toggle = ctx.core_state.clone();
    window.on_toggle_confirm_close_tab(move |enabled| {
        if let Some(w) = window_weak_toggle.upgrade() {
            w.set_setting_confirm_close_tab(enabled);
            let _ = core_state_toggle.storage().config().update(Box::new(move |c| {
                c.confirm_close_tab = enabled;
            }));
            tracing::info!(target: "smagical_ui::settings", "关闭标签页时防误触确认设置为: {}", enabled);
        }
    });




    // -------------------------------------------------------------------------
    // 1.2 复制指定 Tab 会话对应的主机 IP / 连接地址
    // -------------------------------------------------------------------------
    let pane_groups_copy_ip = Rc::clone(&ctx.pane_groups);
    window.on_copy_tab_ip(move |tab_id| {
        let t_id = tab_id.to_string();
        let groups = pane_groups_copy_ip.borrow();
        let mut found_ip = None;
        let mut host_name = String::new();

        for g in groups.iter() {
            if let Some(s) = g.tabs.iter().find(|t| t.session_id == t_id) {
                // 如果地址中包含端口 (如 192.168.1.1:22)，则优先提取纯 IP，纯本地 Shell 保留 127.0.0.1
                let raw_addr = &s.host_address;
                let clean_ip = if raw_addr.starts_with("Local") || raw_addr.is_empty() {
                    "127.0.0.1".to_string()
                } else if let Some((ip_part, _)) = raw_addr.split_once(':') {
                    ip_part.to_string()
                } else {
                    raw_addr.clone()
                };
                found_ip = Some(clean_ip);
                host_name = s.host_name.clone();
                break;
            }
        }

        if let Some(ip) = found_ip
            && let Ok(mut cb) = arboard::Clipboard::new()
        {
            let _ = cb.set_text(ip.clone());
            tracing::info!(target: "smagical_ui::session", "已将会话 [{}] 对应主机 [{}] 的 IP [{}] 复制到剪贴板", t_id, host_name, ip);
        }
    });



    // -------------------------------------------------------------------------
    // 1.3 关闭指定窗格内除目标 Tab 以外的其他会话 (Close Other Tabs)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_close_others = Rc::clone(&ctx.pane_groups);
    let active_pane_id_close_others = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_close_others = Rc::clone(&ctx.global_split_tree);
    let active_terminals_close_others = Rc::clone(&ctx.active_terminals);
    let ctx_close_others = ctx.clone();
    window.on_close_other_tabs(move |pane_id, tab_id| {
        if let Some(w) = window_weak.upgrade() {
            let p_id = pane_id.to_string();
            let t_id = tab_id.to_string();
            let mut groups = pane_groups_close_others.borrow_mut();
            let active_pid = active_pane_id_close_others.borrow();
            let split_tree = global_split_tree_close_others.borrow();

            let target_group_idx = groups.iter().position(|g| g.pane_id == p_id)
                .or_else(|| groups.iter().position(|g| g.tabs.iter().any(|t| t.session_id == t_id)));

            if let Some(idx) = target_group_idx {
                let g = &mut groups[idx];
                let mut removed_tabs = Vec::new();
                g.tabs.retain(|t| {
                    if t.session_id == t_id {
                        true
                    } else {
                        removed_tabs.push(t.session_id.clone());
                        false
                    }
                });
                g.active_tab_id = t_id.clone();

                // 批量清理被移除会话的底层进程并准备快照数据
                let mut terminals = active_terminals_close_others.borrow_mut();
                let mut persist_items = Vec::new();
                for rem_id in removed_tabs {
                    if let Some(mut inst) = terminals.remove(&rem_id) {
                        let snap = inst.snapshot_text(500);
                        let _ = inst.pty.kill();
                        let snap_opt = if !snap.trim().is_empty() {
                            Some((snap.lines().count() as u32, snap))
                        } else {
                            None
                        };
                        persist_items.push((rem_id, snap_opt));
                    }
                }
                drop(terminals);

                let storage_async = ctx_close_others.core_state.storage().clone();
                let window_weak_async = window_weak.clone();
                let search_q = ctx_close_others.history_search_query.borrow().clone();
                let view_mode = ctx_close_others.history_view_mode.borrow().clone();
                let collapsed = ctx_close_others.collapsed_history_groups.borrow().clone();

                ctx_close_others.persistence_guard.spawn(move || {
                    for (rem_id, snap_opt) in persist_items {
                        if let Some((lines_count, snap)) = snap_opt {
                            let hist_id = format!("hist-{}", rem_id);
                            let _ = storage_async.history().save_snapshot(&hist_id, &snap, 500);
                            if let Ok(Some(mut hist)) = storage_async.history().get_by_id(&hist_id) {
                                hist.record_snapshot(lines_count);
                                let _ = storage_async.history().save(&hist);
                            }
                        }
                    }
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w_ui) = window_weak_async.upgrade() {
                            crate::handlers::history_handlers::sync_ui_history_from_state(
                                &w_ui,
                                storage_async.as_ref(),
                                &search_q,
                                &view_mode,
                                &collapsed,
                            );
                        }
                    });
                });
            }


            let is_split = split_tree.is_some();
            sync_active_session_ui(&w, &groups, &active_pid, is_split);
            tracing::info!(target: "smagical_ui::session", "已在窗格 [{}] 关闭其他会话，保留: {}", p_id, t_id);

        }
    });




    // -------------------------------------------------------------------------
    // 2. 切换激活 Tab 会话回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_select = Rc::clone(&ctx.pane_groups);
    let active_pane_id_select = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_select = Rc::clone(&ctx.global_split_tree);
    let core_state_select = ctx.core_state.clone();

    window.on_select_tab(move |sess_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = sess_id.to_string();
            let mut groups = pane_groups_select.borrow_mut();
            let mut active_pid = active_pane_id_select.borrow_mut();
            let is_split = global_split_tree_select.borrow().is_some();

            for g in groups.iter_mut() {
                if g.tabs.iter().any(|t| t.session_id == id_str) {
                    g.active_tab_id = id_str.clone();
                    *active_pid = g.pane_id.clone();
                    break;
                }
            }

            sync_active_session_ui(&w, &groups, &active_pid, is_split);
            crate::session::sync_active_session_to_core(&groups, &active_pid, &core_state_select);
            tracing::debug!(target: "smagical_ui::session", "切换至终端会话: {}", id_str);
        }
    });

    // -------------------------------------------------------------------------
    // 2.1 切换指定分屏窗格内的激活 Tab 会话回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_select_pane_tab = Rc::clone(&ctx.pane_groups);
    let active_pane_id_select_pane_tab = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_select_pane_tab = Rc::clone(&ctx.global_split_tree);
    let core_state_select_pane_tab = ctx.core_state.clone();
    window.on_select_pane_tab(move |pane_id, tab_id| {
        if let Some(w) = window_weak.upgrade() {
            let p_id = pane_id.to_string();
            let t_id = tab_id.to_string();
            let mut groups = pane_groups_select_pane_tab.borrow_mut();
            let mut active_pid = active_pane_id_select_pane_tab.borrow_mut();
            let is_split = global_split_tree_select_pane_tab.borrow().is_some();

            if let Some(g) = groups.iter_mut().find(|g| g.pane_id == p_id) {
                g.active_tab_id = t_id.clone();
                *active_pid = p_id.clone();
            }

            sync_active_session_ui(&w, &groups, &active_pid, is_split);
            crate::session::sync_active_session_to_core(&groups, &active_pid, &core_state_select_pane_tab);
            tracing::debug!(target: "smagical_ui::session", "窗格 [{}] 切换至 Tab: {}", p_id, t_id);
        }
    });

    // -------------------------------------------------------------------------
    // 2.2 终端 Tab 拖拽重排与跨分屏移动/自动合并回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_move_tab = Rc::clone(&ctx.pane_groups);
    let active_pane_id_move_tab = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_move_tab = Rc::clone(&ctx.global_split_tree);
    let core_state_move_tab = ctx.core_state.clone();
    window.on_move_terminal_tab(move |from_pane_id, from_index, drop_x, drop_y| {
        if let Some(w) = window_weak.upgrade() {
            let from_pid = from_pane_id.to_string();
            let from_idx = from_index as usize;
            let drop_x_px = drop_x;
            let drop_y_px = drop_y;

            let mut groups = pane_groups_move_tab.borrow_mut();
            let mut active_pid = active_pane_id_move_tab.borrow_mut();
            let mut split_tree = global_split_tree_move_tab.borrow_mut();

            if groups.is_empty() {
                return;
            }

            // 1. 定位源窗格
            let src_idx_opt = groups.iter().position(|g| g.pane_id == from_pid || (from_pid.is_empty() && g.pane_id == *active_pid)).or_else(|| groups.iter().position(|g| !g.tabs.is_empty()));
            let src_idx = match src_idx_opt {
                Some(idx) => idx,
                None => return,
            };

            if from_idx >= groups[src_idx].tabs.len() {
                return;
            }

            let src_pane_id = groups[src_idx].pane_id.clone();

            // 2. 根据分屏模式推导落点目标窗格与插入位置
            let (to_pid, insert_pos) = if let Some(tree) = split_tree.as_ref() {
                let vp_w = w.get_terminal_canvas_width().max(200.0);
                let vp_h = w.get_terminal_canvas_height().max(100.0);
                let (panes_layout, _) = tree.compute_pixel_layout(vp_w, vp_h, 2.0, None);

                let target_layout = panes_layout.iter().find(|p| {
                    drop_x_px >= p.x && drop_x_px <= p.x + p.width && drop_y_px >= p.y && drop_y_px <= p.y + p.height
                });

                if let Some(pl) = target_layout {
                    let rel_x = (drop_x_px - (pl.x + 6.0)).max(0.0);
                    let to_i = (rel_x / 152.0).round() as usize;
                    (pl.pane_id.clone(), to_i)
                } else {
                    let rel_x = (drop_x_px - 6.0).max(0.0);
                    let to_i = (rel_x / 152.0).round() as usize;
                    (src_pane_id.clone(), to_i)
                }
            } else {
                let rel_x = (drop_x_px - 6.0).max(0.0);
                let to_i = (rel_x / 152.0).round() as usize;
                (src_pane_id.clone(), to_i)
            };

            // 3. 判断是否在同窗格内重排
            if to_pid == src_pane_id {
                let tab = groups[src_idx].tabs.remove(from_idx);
                let clamped_pos = insert_pos.min(groups[src_idx].tabs.len());
                groups[src_idx].tabs.insert(clamped_pos, tab.clone());
                groups[src_idx].active_tab_id = tab.session_id.clone();
                *active_pid = src_pane_id.clone();

                let is_split = split_tree.is_some();
                sync_active_session_ui(&w, &groups, &active_pid, is_split);
                crate::session::sync_active_session_to_core(&groups, &active_pid, &core_state_move_tab);
                tracing::info!(
                    target: "smagical_ui::session",
                    "窗格 [{}] 内 Tab [{}] 重排完成: 索引 {} -> {}",
                    src_pane_id, tab.session_id, from_idx, clamped_pos
                );
                return;
            }

            // 4. 跨分屏移动 (从 src_pane_id 移动到 to_pid)
            let moved_tab = groups[src_idx].tabs.remove(from_idx);
            let src_is_now_empty = groups[src_idx].tabs.is_empty();

            if !src_is_now_empty
                && groups[src_idx].active_tab_id == moved_tab.session_id
            {
                let next_pos = if from_idx > 0 { from_idx - 1 } else { 0 };
                groups[src_idx].active_tab_id = groups[src_idx].tabs[next_pos.min(groups[src_idx].tabs.len() - 1)].session_id.clone();
            }

            let tgt_idx_opt = groups.iter().position(|g| g.pane_id == to_pid);
            let tgt_idx = match tgt_idx_opt {
                Some(idx) => idx,
                None => {
                    // 目标窗格不存在时插回源窗格并恢复
                    let cur_len = groups[src_idx].tabs.len();
                    groups[src_idx].tabs.insert(from_idx.min(cur_len), moved_tab);
                    return;
                }
            };

            let clamped_insert = insert_pos.min(groups[tgt_idx].tabs.len());
            groups[tgt_idx].tabs.insert(clamped_insert, moved_tab.clone());
            groups[tgt_idx].active_tab_id = moved_tab.session_id.clone();
            *active_pid = to_pid.clone();

            // 5. 核心：如果源窗格最后一个 Tab 迁出，自动关闭并合并原分屏窗格
            if src_is_now_empty {
                let empty_pos = groups.iter().position(|g| g.pane_id == src_pane_id);
                if let Some(pos) = empty_pos {
                    groups.remove(pos);
                }

                if let Some(tree) = split_tree.as_mut() {
                    tree.close_pane(&src_pane_id);
                    if tree.leaf_count() <= 1 {
                        *split_tree = None;
                    }
                }
                tracing::info!(
                    target: "smagical_ui::session",
                    "源分屏窗格 [{}] 所有 Tab 均已迁出，已自动关闭并合并原窗格",
                    src_pane_id
                );
            }

            let is_split = split_tree.is_some();
            sync_active_session_ui(&w, &groups, &active_pid, is_split);
            crate::session::sync_active_session_to_core(&groups, &active_pid, &core_state_move_tab);
            core_state_move_tab.events().dispatch(&TerminalSplitChangedEvent {
                group_count: groups.len(),
                active_pane_id: to_pid.clone(),
                is_split,
            });

            tracing::info!(
                target: "smagical_ui::session",
                "跨分屏移动 Tab [{}] 成功: 从窗格 [{}] 迁移至窗格 [{}] (目标位置: {}, 剩余分屏数: {})",
                moved_tab.session_id, src_pane_id, to_pid, clamped_insert, groups.len()
            );
        }
    });

    // -------------------------------------------------------------------------
    // 3. 呼出新建终端会话弹窗回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    window.on_new_tab(move || {
        if let Some(_w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::session", "呼出快速新建会话中心");
        }
    });


    // -------------------------------------------------------------------------
    // 4. 快速新建会话中心实时关键词搜索过滤回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let master_tree_launcher = Rc::clone(&ctx.master_tree);
    let cached_shells_launcher = std::sync::Arc::clone(&ctx.cached_shells);
    window.on_filter_launcher(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let q = query.trim().to_lowercase();
            let all_cached = cached_shells_launcher.read().unwrap();

            let filtered_locals: Vec<LocalShellItemData> = if q.is_empty() {
                all_cached.clone()
            } else {
                all_cached
                    .iter()
                    .filter(|s| {
                        let t = s.title.to_lowercase();
                        let sub = s.subtitle.to_lowercase();
                        let id = s.id.to_lowercase();
                        let tag = s.tag.to_lowercase();
                        t.contains(&q) || sub.contains(&q) || id.contains(&q) || tag.contains(&q)
                            || (q.contains("wsl") && (id.contains("wsl") || sub.contains("wsl")))
                            || (q.contains("ps") && (id.contains("powershell") || id.contains("pwsh")))
                            || (q.contains("bash") && (id.contains("bash") || id.contains("wsl")))
                            || (q.contains("zsh") && id.contains("zsh"))
                            || (q.contains("fish") && id.contains("fish"))
                            || (q.contains("cmd") && id.contains("cmd"))
                    })
                    .cloned()
                    .collect()
            };
            w.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(filtered_locals))));

            let tree = master_tree_launcher.borrow();
            let filtered_hosts: Vec<HostItemData> = tree
                .iter()
                .filter(|n| {
                    if n.is_group {
                        false
                    } else if q.is_empty() {
                        true
                    } else {
                        n.name.to_lowercase().contains(&q)
                            || n.address.to_lowercase().contains(&q)
                            || n.parent_id.to_lowercase().contains(&q)
                    }
                })
                .map(|n| HostItemData {
                    id: n.id.clone().into(),
                    name: n.name.clone().into(),
                    address: n.address.clone().into(),
                    port: n.port,
                    group: "".into(),
                    status: n.status.clone().into(),
                    ping_ms: n.ping_ms,
                })
                .collect();
            w.set_launcher_host_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(filtered_hosts))));
        }
    });



    // -------------------------------------------------------------------------
    // 5. 快捷指令片段发送回调
    // -------------------------------------------------------------------------
    let pane_groups_snippet = Rc::clone(&ctx.pane_groups);
    let active_pane_id_snippet = Rc::clone(&ctx.active_pane_id);
    let active_terminals_snippet = Rc::clone(&ctx.active_terminals);
    window.on_send_snippet(move |cmd| {
        let active_pid = active_pane_id_snippet.borrow().clone();
        let groups = pane_groups_snippet.borrow();
        if let Some(g) = groups.iter().find(|g| g.pane_id == active_pid).or_else(|| groups.first())
            && let Some(active_sess) = g.get_active_session()
        {
            let mut terminals = active_terminals_snippet.borrow_mut();
            if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                let cmd_str = format!("{}\n", cmd);
                let _ = instance.send_input(&cmd_str);
            }
            tracing::info!(target: "smagical_ui::cmd", "向终端发送指令片段: {}", cmd);
        }
    });

    // -------------------------------------------------------------------------
    // 6. 终端按键输入捕获回调
    // -------------------------------------------------------------------------
    let pane_groups_input = Rc::clone(&ctx.pane_groups);
    let active_pane_id_input = Rc::clone(&ctx.active_pane_id);
    let active_terminals_input = Rc::clone(&ctx.active_terminals);
    window.on_terminal_key_input(move |text, is_ctrl, is_shift, is_alt| {
        let active_pid = active_pane_id_input.borrow().clone();
        let groups = pane_groups_input.borrow();
        if let Some(g) = groups.iter().find(|g| g.pane_id == active_pid).or_else(|| groups.first())
            && let Some(active_sess) = g.get_active_session()
        {
            let mut terminals = active_terminals_input.borrow_mut();
            if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                if is_shift && (text == "\u{0012}" || text == "PageUp") {
                    instance.scroll_page_up();
                    return;
                }
                if is_shift && (text == "\u{0013}" || text == "PageDown") {
                    instance.scroll_page_down();
                    return;
                }

                instance.scroll_to_bottom();
                let bytes = encode_key_event(text.as_str(), is_ctrl, is_shift, is_alt);
                let _ = instance.send_bytes(&bytes);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 7. 终端滚轮视口滚动回调
    // -------------------------------------------------------------------------
    let pane_groups_scroll = Rc::clone(&ctx.pane_groups);
    let active_pane_id_scroll = Rc::clone(&ctx.active_pane_id);
    let active_terminals_scroll = Rc::clone(&ctx.active_terminals);
    let scroll_accum = Rc::new(std::cell::Cell::new(0.0f32));
    window.on_terminal_scroll(move |delta| {
        let active_pid = active_pane_id_scroll.borrow().clone();
        let groups = pane_groups_scroll.borrow();
        if let Some(g) = groups.iter().find(|g| g.pane_id == active_pid).or_else(|| groups.first())
            && let Some(active_sess) = g.get_active_session()
        {
            let current = scroll_accum.get() + delta;
            let line_step = 40.0f32;
            let lines = (current / line_step) as i32;

            if lines != 0 {
                scroll_accum.set(current - (lines as f32) * line_step);
                let mut terminals = active_terminals_scroll.borrow_mut();
                if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                    instance.scroll_delta(lines);
                }
            } else {
                scroll_accum.set(current);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 8. 终端选区复制到剪贴板回调
    // -------------------------------------------------------------------------
    let pane_groups_copy = Rc::clone(&ctx.pane_groups);
    let active_pane_id_copy = Rc::clone(&ctx.active_pane_id);
    let active_terminals_copy = Rc::clone(&ctx.active_terminals);
    window.on_terminal_copy(move || {
        let active_pid = active_pane_id_copy.borrow().clone();
        let groups = pane_groups_copy.borrow();
        if let Some(g) = groups.iter().find(|g| g.pane_id == active_pid).or_else(|| groups.first())
            && let Some(active_sess) = g.get_active_session()
        {
            let mut terminals = active_terminals_copy.borrow_mut();
            if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                let text = instance.parser.copy_selection_text();
                if !text.is_empty()
                    && let Ok(mut clipboard) = arboard::Clipboard::new()
                {
                    let _ = clipboard.set_text(text);
                }
            }
            tracing::info!(target: "smagical_ui::terminal", "执行终端 [{}] 选区复制", active_sess.session_id);
        }
    });

    // -------------------------------------------------------------------------
    // 8.1 终端鼠标拖拽划选选区变更回调
    // -------------------------------------------------------------------------
    let pane_groups_sel = Rc::clone(&ctx.pane_groups);
    let active_pane_id_sel = Rc::clone(&ctx.active_pane_id);
    let active_terminals_sel = Rc::clone(&ctx.active_terminals);
    let core_state_sel = ctx.core_state.clone();
    window.on_terminal_selection_changed(move |sc, sr, ec, er, has_sel| {
        let active_pid = active_pane_id_sel.borrow().clone();
        let groups = pane_groups_sel.borrow();
        if let Some(g) = groups.iter().find(|g| g.pane_id == active_pid).or_else(|| groups.first())
            && let Some(active_sess) = g.get_active_session()
        {
            let mut terminals = active_terminals_sel.borrow_mut();
            if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                if has_sel && sc >= 0 && sr >= 0 && ec >= 0 && er >= 0 {
                    instance.parser.set_selection((sc as usize, sr as usize), (ec as usize, er as usize));
                    if let Ok(cfg) = core_state_sel.storage().config().get() {
                        if cfg.copy_on_select {
                            let text = instance.parser.copy_selection_text();
                            if !text.is_empty() {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(text);
                                }
                            }
                        }
                    }
                } else {
                    instance.parser.clear_selection();
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 9. 剪贴板文本粘贴到终端回调
    // -------------------------------------------------------------------------
    let pane_groups_paste = Rc::clone(&ctx.pane_groups);
    let active_pane_id_paste = Rc::clone(&ctx.active_pane_id);
    let active_terminals_paste = Rc::clone(&ctx.active_terminals);
    let core_state_paste = ctx.core_state.clone();
    let notif_paste = ctx.notifications.clone();
    window.on_terminal_paste(move || {
        let active_pid = active_pane_id_paste.borrow().clone();
        let groups = pane_groups_paste.borrow();
        if let Some(g) = groups.iter().find(|g| g.pane_id == active_pid).or_else(|| groups.first())
            && let Some(active_sess) = g.get_active_session()
        {
            if let Ok(text) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                if let Ok(cfg) = core_state_paste.storage().config().get() {
                    if cfg.warn_on_multiline_paste && (text.contains('\n') || text.contains('\r')) {
                        let lines_count = text.lines().count();
                        if lines_count > 1 {
                            notif_paste.warning(
                                "多行安全粘贴提示",
                                &format!("已向终端安全写入包含 {} 行的命令/文本", lines_count),
                            );
                        }
                    }
                }
                let mut terminals = active_terminals_paste.borrow_mut();
                if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                    let _ = instance.send_input(&text);
                }
            }
            tracing::info!(target: "smagical_ui::terminal", "执行剪贴板粘贴到终端 [{}]", active_sess.session_id);
        }
    });

    // -------------------------------------------------------------------------
    // 10. 终端清屏回调
    // -------------------------------------------------------------------------
    let pane_groups_clear = Rc::clone(&ctx.pane_groups);
    let active_pane_id_clear = Rc::clone(&ctx.active_pane_id);
    let active_terminals_clear = Rc::clone(&ctx.active_terminals);
    window.on_terminal_clear(move || {
        let active_pid = active_pane_id_clear.borrow().clone();
        let groups = pane_groups_clear.borrow();
        if let Some(g) = groups.iter().find(|g| g.pane_id == active_pid).or_else(|| groups.first())
            && let Some(active_sess) = g.get_active_session()
        {
            let mut terminals = active_terminals_clear.borrow_mut();
            if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                let _ = instance.clear();
            }
            tracing::info!(target: "smagical_ui::terminal", "执行终端 [{}] 清屏", active_sess.session_id);
        }
    });



    // -------------------------------------------------------------------------
    // 11. 终端多窗格分屏回调 (Editor Group 优雅切分模型)
    // -------------------------------------------------------------------------
    // 当在分屏窗格中触发分屏时：
    // - 仅当当前窗格存在至少 2 个 Tab 时支持分屏，将当前激活的 Tab 迁移到新分屏窗格；
    // - 若当前窗格仅有 1 个 Tab，则不支持分屏（无多余 Tab 可分过去）。
    let window_weak = window.as_weak();
    let pane_groups_split = Rc::clone(&ctx.pane_groups);
    let active_pane_id_split = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_split = Rc::clone(&ctx.global_split_tree);
    let next_pane_num_split = Rc::clone(&ctx.next_pane_num);
    let core_state_split = ctx.core_state.clone();
    window.on_split_terminal(move |orient| {
        if let Some(w) = window_weak.upgrade() {
            let mut groups = pane_groups_split.borrow_mut();
            let mut active_pid = active_pane_id_split.borrow_mut();
            if groups.is_empty() {
                return;
            }

            let target_idx = groups.iter().position(|g| g.pane_id == *active_pid).unwrap_or(0);
            if groups[target_idx].tabs.len() <= 1 {
                tracing::warn!(target: "smagical_ui::session", "当前窗格仅有 1 个会话 Tab，无法分屏至新窗格 (分屏需要从当前窗格迁移 Tab，至少需要 2 个 Tab)");
                return;
            }

            let split_dir = match orient.as_str() {
                "horizontal" => SplitOrientation::Horizontal,
                _ => SplitOrientation::Vertical,
            };

            let target_pane_id = groups[target_idx].pane_id.clone();
            let mut pane_counter = next_pane_num_split.borrow_mut();
            let new_pane_id = format!("pane-{}", *pane_counter);
            *pane_counter += 1;

            // 将当前激活 Tab 移入新窗格
            let active_tab_id = groups[target_idx].active_tab_id.clone();
            let tab_pos = groups[target_idx].tabs.iter().position(|t| t.session_id == active_tab_id).unwrap_or(0);
            let moved_session = groups[target_idx].tabs.remove(tab_pos);

            let remaining_act_pos = if tab_pos > 0 { tab_pos - 1 } else { 0 };
            groups[target_idx].active_tab_id = groups[target_idx].tabs[remaining_act_pos.min(groups[target_idx].tabs.len() - 1)].session_id.clone();

            let new_group = PaneGroup::new_single(new_pane_id.clone(), moved_session);
            groups.push(new_group);

            // 更新全局二叉树拓扑
            let mut split_tree = global_split_tree_split.borrow_mut();
            if split_tree.is_none() {
                let mut tree = SplitNode::new_single(target_pane_id.clone(), "Pane 1".to_string());
                tree.split_pane(&target_pane_id, new_pane_id.clone(), "Pane 2".to_string(), split_dir);
                *split_tree = Some(tree);
            } else if let Some(tree) = split_tree.as_mut() {
                tree.split_pane(&target_pane_id, new_pane_id.clone(), format!("Pane {}", *pane_counter), split_dir);
            }

            *active_pid = new_pane_id.clone();
            sync_active_session_ui(&w, &groups, &new_pane_id, true);
            core_state_split.events().dispatch(&TerminalSplitChangedEvent {
                group_count: groups.len(),
                active_pane_id: new_pane_id.clone(),
                is_split: true,
            });

            tracing::info!(target: "smagical_ui::session", "成功在窗格 [{}] 上切分新分屏窗格 [{}] (总窗格数: {})", target_pane_id, new_pane_id, groups.len());
        }
    });



    // -------------------------------------------------------------------------
    // 12. 按窗格 ID 关闭指定分屏窗格回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_close_id = Rc::clone(&ctx.pane_groups);
    let active_pane_id_close_id = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_close_id = Rc::clone(&ctx.global_split_tree);
    let active_terminals_close_id = Rc::clone(&ctx.active_terminals);
    let ctx_close_pane_id = ctx.clone();
    window.on_close_pane_by_id(move |target_pane_id| {
        if let Some(w) = window_weak.upgrade() {
            let pid = target_pane_id.to_string();
            let mut groups = pane_groups_close_id.borrow_mut();
            let mut active_pid = active_pane_id_close_id.borrow_mut();
            let mut split_tree = global_split_tree_close_id.borrow_mut();

            if let Some(idx) = groups.iter().position(|g| g.pane_id == pid) {
                let closed_group = groups.remove(idx);
                let mut terminals = active_terminals_close_id.borrow_mut();
                for t in closed_group.tabs {
                    if let Some(mut inst) = terminals.remove(&t.session_id) {
                        let snap = inst.snapshot_text(500);
                        let _ = inst.pty.kill();
                        let hist_id = format!("hist-{}", t.session_id);
                        if !snap.trim().is_empty() {
                            let _ = ctx_close_pane_id.core_state.storage().history().save_snapshot(&hist_id, &snap, 500);
                            if let Ok(Some(mut hist)) = ctx_close_pane_id.core_state.storage().history().get_by_id(&hist_id) {
                                hist.record_snapshot(snap.lines().count() as u32);
                                let _ = ctx_close_pane_id.core_state.storage().history().save(&hist);
                            }
                        }
                        ctx_close_pane_id.core_state.events().dispatch(&TerminalSessionEvent {
                            session_id: t.session_id.clone(),
                            host_id: "".into(),
                            action: "closed".into(),
                        });
                    }
                }
                crate::handlers::history_handlers::sync_ui_history(&w, &ctx_close_pane_id);

            }

            if let Some(tree) = split_tree.as_mut() {
                tree.close_pane(&pid);
                if tree.leaf_count() <= 1 {
                    *split_tree = None;
                }
            }

            if !groups.is_empty() {
                *active_pid = groups[0].pane_id.clone();
            } else {
                *active_pid = String::new();
            }

            let is_split = split_tree.is_some();
            sync_active_session_ui(&w, &groups, &active_pid, is_split);
            ctx_close_pane_id.core_state.events().dispatch(&TerminalSplitChangedEvent {
                group_count: groups.len(),
                active_pane_id: active_pid.clone(),
                is_split,
            });
            tracing::info!(target: "smagical_ui::session", "已关闭窗格: {}", pid);

        }
    });


    // -------------------------------------------------------------------------
    // 12.1 按窗格下标关闭分屏窗格回调 (向后兼容)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_close_idx = Rc::clone(&ctx.pane_groups);
    let active_pane_id_close_idx = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_close_idx = Rc::clone(&ctx.global_split_tree);
    let active_terminals_close_idx = Rc::clone(&ctx.active_terminals);
    window.on_close_pane_by_index(move |idx| {
        if let Some(w) = window_weak.upgrade() {
            let mut groups = pane_groups_close_idx.borrow_mut();
            let mut active_pid = active_pane_id_close_idx.borrow_mut();
            let mut split_tree = global_split_tree_close_idx.borrow_mut();

            let u_idx = idx as usize;
            if u_idx < groups.len() {
                let closed_group = groups.remove(u_idx);
                let pid = closed_group.pane_id.clone();
                let mut terminals = active_terminals_close_idx.borrow_mut();
                for t in closed_group.tabs {
                    if let Some(mut inst) = terminals.remove(&t.session_id) {
                        let _ = inst.pty.kill();
                    }
                }

                if let Some(tree) = split_tree.as_mut() {
                    tree.close_pane(&pid);
                    if tree.leaf_count() <= 1 {
                        *split_tree = None;
                    }
                }

                if !groups.is_empty() {
                    *active_pid = groups[0].pane_id.clone();
                } else {
                    *active_pid = String::new();
                }

                let is_split = split_tree.is_some();
                sync_active_session_ui(&w, &groups, &active_pid, is_split);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 12.2 关闭当前终端全部嵌套分屏模式 (合并为单屏)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_close_split = Rc::clone(&ctx.pane_groups);
    let active_pane_id_close_split = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_close_split = Rc::clone(&ctx.global_split_tree);
    let core_state_close_split = ctx.core_state.clone();
    window.on_close_split(move || {
        if let Some(w) = window_weak.upgrade() {
            let mut groups = pane_groups_close_split.borrow_mut();
            let mut active_pid = active_pane_id_close_split.borrow_mut();
            *global_split_tree_close_split.borrow_mut() = None;

            if groups.len() > 1 {
                let first_pid = groups[0].pane_id.clone();
                // 将所有后续分屏窗格的 tabs 合并入第一窗格
                let mut all_tabs = Vec::new();
                for (i, g) in groups.drain(..).enumerate() {
                    if i == 0 {
                        all_tabs = g.tabs;
                    } else {
                        for t in g.tabs {
                            all_tabs.push(t);
                        }
                    }
                }
                if let Some(first_tab) = all_tabs.first() {
                    let act_id = first_tab.session_id.clone();
                    groups.push(PaneGroup {
                        pane_id: first_pid.clone(),
                        tabs: all_tabs,
                        active_tab_id: act_id,
                    });
                    *active_pid = first_pid;
                }
            }

            sync_active_session_ui(&w, &groups, &active_pid, false);
            core_state_close_split.events().dispatch(&TerminalSplitChangedEvent {
                group_count: groups.len(),
                active_pane_id: active_pid.clone(),
                is_split: false,
            });
            tracing::info!(target: "smagical_ui::session", "退出分屏模式并合并所有 Tab");
        }
    });


    // -------------------------------------------------------------------------
    // 13. 选择并聚焦指定分屏窗格回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_sel_id = Rc::clone(&ctx.pane_groups);
    let active_pane_id_sel_id = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_sel_id = Rc::clone(&ctx.global_split_tree);
    window.on_select_pane_by_id(move |pane_id| {
        if let Some(w) = window_weak.upgrade() {
            let pid = pane_id.to_string();
            let groups = pane_groups_sel_id.borrow();
            let is_split = global_split_tree_sel_id.borrow().is_some();
            *active_pane_id_sel_id.borrow_mut() = pid.clone();
            sync_active_session_ui(&w, &groups, &pid, is_split);
        }
    });

    let window_weak = window.as_weak();
    window.on_select_pane(move |idx| {
        if let Some(w) = window_weak.upgrade() {
            w.set_active_pane_index(idx);
        }
    });

    // -------------------------------------------------------------------------
    // 13.1 切换单窗格临时最大化 (Zoom) 与还原回调
    // -------------------------------------------------------------------------
    let zoomed_pane_id_toggle = Rc::clone(&ctx.zoomed_pane_id);
    window.on_toggle_pane_zoom(move |pane_id| {
        let target_id = pane_id.to_string();
        let mut zoomed = zoomed_pane_id_toggle.borrow_mut();
        if zoomed.as_deref() == Some(target_id.as_str()) {
            *zoomed = None;
            tracing::info!(target: "smagical_ui::terminal", "还原窗格 [{}] 至多分屏布局", target_id);
        } else {
            *zoomed = Some(target_id.clone());
            tracing::info!(target: "smagical_ui::terminal", "单窗格 [{}] 临时最大化 (Zoom)", target_id);
        }
    });

    // -------------------------------------------------------------------------
    // 13.2 动态拖拽调节分割条比例回调
    // -------------------------------------------------------------------------
    let global_split_tree_adjust = Rc::clone(&ctx.global_split_tree);
    window.on_adjust_splitter(move |splitter_id, delta_ratio| {
        let mut tree_guard = global_split_tree_adjust.borrow_mut();
        if let Some(tree) = tree_guard.as_mut() {
            let _res = tree.adjust_splitter(splitter_id.as_str(), delta_ratio);
        }
    });

    // -------------------------------------------------------------------------
    // 14. 终端内文本查找回调
    // -------------------------------------------------------------------------
    window.on_search_terminal(move |query, match_case| {
        tracing::info!(target: "smagical_ui::terminal", "终端查找文本: {:?} (大小写敏感: {})", query, match_case);
    });

    // -------------------------------------------------------------------------
    // 15. 终端动态几何网格尺寸调节回调 (Resize)
    // -------------------------------------------------------------------------
    let pane_groups_resize = Rc::clone(&ctx.pane_groups);
    let active_pane_id_resize = Rc::clone(&ctx.active_pane_id);
    let active_terminals_resize = Rc::clone(&ctx.active_terminals);
    window.on_terminal_resize(move |cols, rows| {
        let active_pid = active_pane_id_resize.borrow().clone();
        let groups = pane_groups_resize.borrow();
        if let Some(g) = groups.iter().find(|g| g.pane_id == active_pid).or_else(|| groups.first())
            && let Some(active_sess) = g.get_active_session()
        {
            let mut terminals = active_terminals_resize.borrow_mut();
            if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                let _ = instance.resize(cols as u16, rows as u16);
            }
        }
    });
}







