//! smagicalssh UI crate。
//!
//! 这里依赖 `smagical-core`，负责桌面装配、Slint 界面和主题应用。

#![deny(missing_docs)]

/// 本地终端环境探测模块。
pub mod local_shells;

/// Slint 主题资源注册、内置预设和运行时应用接口。
pub mod theme;

/// 主机资产树形数据模型与纯函数操作层。
pub(crate) mod tree_model;

/// 终端会话管理与 Slint UI 同步。
pub(crate) mod session;

/// Debug 日志面板与全局 Tracing 日志同步。
pub(crate) mod debug_ui;

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use slint::{ComponentHandle, Model};
use smagical_core::{CoreState, GroupRecord};
use smagical_debug::{
    generate_batch_hosts, get_preset_by_id, BatchGenerateConfig,
};
use theme::{apply_theme_by_id, initialize_theme_service};

use tree_model::{
    RawTreeNode, build_raw_tree_from_storage, build_group_options,
    build_visible_tree_nodes, build_search_tree_nodes, calculate_max_tree_width,
    ensure_raw_group_hierarchy, move_and_reorder_raw_node,
};
use session::{TerminalSessionInfo, sync_active_session_ui};
use debug_ui::sync_ui_debug_logs;

#[allow(missing_docs, dead_code)]
mod generated {
    slint::include_modules!();
}

pub use generated::{
    AppColorScheme, AppTheme, AppWindow, GroupOptionData, HostItemData, HostTreeNode,
    LocalShellItemData, LogEntryData, TabData,
};
/// 创建并运行桌面应用主窗口。
pub fn run() -> anyhow::Result<()> {
    let window = AppWindow::new()?;
    let themes = Rc::new(initialize_theme_service(None)?);

    // 默认应用 Darcula 主题
    apply_theme_by_id(&window, &themes, "builtin.ui.darcula")?;
    window.set_current_theme_name("Darcula".into());

    // 活跃会话管理状态 (初始清空全部 Tab)
    let active_sessions: Rc<RefCell<Vec<TerminalSessionInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let next_session_num: Rc<RefCell<usize>> = Rc::new(RefCell::new(1));
    sync_active_session_ui(&window, &active_sessions.borrow(), "");

    // [P2 Fix] 启动时一次性探测本地 Shell 环境并缓存，避免每次搜索都重复扫描文件系统
    let cached_shells = Rc::new(local_shells::detect_local_shells());
    window.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from((*cached_shells).clone()))));


    tracing::info!(target: "smagical_ui", "Smalux-SSH 桌面应用工作台就绪");
    sync_ui_debug_logs(&window);

    // 绑定主题切换回调
    let window_weak = window.as_weak();
    let themes_clone = Rc::clone(&themes);
    window.on_switch_theme(move |theme_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = theme_id.as_str();
            let _ = apply_theme_by_id(&w, &themes_clone, id_str);
            let name = if id_str.contains("darcula") {
                "Darcula"
            } else if id_str.contains("monokai") {
                "Monokai"
            } else if id_str.contains("onedark") || id_str.contains("one-dark") {
                "One Dark"
            } else if id_str.contains("solarized-light") {
                "Solarized Light"
            } else if id_str.contains("solarized") {
                "Solarized"
            } else if id_str.contains("github-light") || id_str.contains("light") {
                "GitHub Light"
            } else if id_str.contains("github-dark") {
                "GitHub Dark"
            } else {
                "System"
            };
            w.set_current_theme_name(name.into());

            // 自动同步深浅色状态
            let is_light = id_str.contains("light") || id_str.contains("dawn") || id_str.contains("latte");
            w.set_is_dark_mode(!is_light);

            tracing::info!(target: "smagical_ui::theme", "切换应用配色主题: {} ({})", name, id_str);
            sync_ui_debug_logs(&w);
        }
    });

    // 绑定深色 / 浅色模式一键切换回调
    let window_weak = window.as_weak();
    let themes_clone = Rc::clone(&themes);
    window.on_toggle_color_mode(move || {
        if let Some(w) = window_weak.upgrade() {
            let is_dark = w.get_is_dark_mode();
            let next_dark = !is_dark;
            w.set_is_dark_mode(next_dark);

            if next_dark {
                let _ = apply_theme_by_id(&w, &themes_clone, "builtin.ui.darcula");
                w.set_current_theme_name("Darcula".into());
            } else {
                let _ = apply_theme_by_id(&w, &themes_clone, "builtin.ui.github-light");
                w.set_current_theme_name("GitHub Light".into());
            }

            tracing::info!(target: "smagical_ui::theme", "{}", if next_dark { "切换至深色模式 (Darcula)" } else { "切换至浅色模式 (GitHub Light)" });
            sync_ui_debug_logs(&w);
        }
    });

    // 绑定关闭 Tab 回调 (实时从列表中移除该会话，并智能切换至邻近 Tab)
    let window_weak = window.as_weak();
    let active_sessions_close = Rc::clone(&active_sessions);
    window.on_close_tab(move |sess_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = sess_id.to_string();
            let mut sessions = active_sessions_close.borrow_mut();
            let cur_active = w.get_active_session_tab().to_string();

            let mut next_active = cur_active.clone();
            if let Some(idx) = sessions.iter().position(|s| s.session_id == id_str) {
                if cur_active == id_str {
                    if idx > 0 {
                        next_active = sessions[idx - 1].session_id.clone();
                    } else if idx + 1 < sessions.len() {
                        next_active = sessions[idx + 1].session_id.clone();
                    } else {
                        next_active = "".to_string();
                    }
                }
                sessions.remove(idx);
            }

            sync_active_session_ui(&w, &sessions, &next_active);
            tracing::info!(target: "smagical_ui::session", "已关闭终端会话: {}", id_str);
            sync_ui_debug_logs(&w);
        }
    });

    // 绑定切换 Tab 回调 (点击 Tab 时激活对应的会话)
    let window_weak = window.as_weak();
    let active_sessions_select = Rc::clone(&active_sessions);
    window.on_select_tab(move |sess_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = sess_id.to_string();
            let sessions = active_sessions_select.borrow();
            sync_active_session_ui(&w, &sessions, &id_str);
            tracing::debug!(target: "smagical_ui::session", "切换至终端会话: {}", id_str);
        }
    });

    // 初始化 CoreState 核心状态引擎 (基于 MockStorage 预设种子存储)
    let core_state = Rc::new(CoreState::new_mock());

    // 从存储层读取初始主控树形结构与分组生成器
    let initial_tree = build_raw_tree_from_storage(core_state.storage().as_ref());
    let master_tree = Rc::new(RefCell::new(initial_tree));
    let next_group_id = Rc::new(RefCell::new(100));

    // 从存储层初始化树形结构折叠状态 (读取所有 is_expanded == true 的分组)
    let initial_expanded: HashSet<String> = core_state
        .storage()
        .groups()
        .list_all()
        .unwrap_or_default()
        .into_iter()
        .filter(|g| g.is_expanded)
        .map(|g| g.id)
        .collect();
    let expanded_groups = Rc::new(RefCell::new(initial_expanded));

    let search_query = Rc::new(RefCell::new(String::new()));

    // [P1 Fix] 动态初始化上级分组选择器展开状态：从存储中读取所有顶级分组（parent_id 为 None），
    // 替代之前硬编码的 "grp-prod"，确保切换存储后端后仍然正确工作。
    let mut initial_selector_expanded = HashSet::from(["root".to_string()]);
    core_state.storage().groups().list_all().unwrap_or_default()
        .into_iter()
        .filter(|g| g.parent_id.is_none())
        .for_each(|g| { initial_selector_expanded.insert(g.id); });
    let selector_expanded_groups = Rc::new(RefCell::new(initial_selector_expanded));


    // 初始渲染上级分组选项数据
    let initial_options = build_group_options(&master_tree.borrow(), &selector_expanded_groups.borrow());
    window.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(initial_options))));

    // 初始渲染树形节点
    let initial_nodes = build_visible_tree_nodes(&master_tree.borrow(), &expanded_groups.borrow());
    window.set_tree_content_width(calculate_max_tree_width(&initial_nodes));
    window.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(initial_nodes))));

    // 从存储层初始渲染卡片列表 (全量主机资产，供卡片模式纵向滚动测试)
    let all_hosts = core_state.storage().hosts().list_all().unwrap_or_default();
    let all_groups = core_state.storage().groups().list_all().unwrap_or_default();
    let initial_cards: Vec<HostItemData> = all_hosts
        .into_iter()
        .map(|h| {
            let group_name = h.parent_group_id.as_deref().and_then(|p_id| {
                all_groups.iter().find(|g| g.id == p_id).map(|g| g.name.clone())
            }).unwrap_or_else(|| "未分组".to_string());
            HostItemData {
                id: h.id.into(),
                name: h.name.into(),
                address: h.address.into(),
                port: h.port as i32,
                group: group_name.into(),
                status: h.status.to_string().into(),
                ping_ms: h.ping_ms,
            }
        })
        .collect();
    let master_cards = Rc::new(RefCell::new(initial_cards.clone()));
    window.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(initial_cards))));

    // 绑定上级分组选择器折叠 / 展开回调 (支持弹窗内自由收缩/展开子节点)
    let window_weak = window.as_weak();
    let master_tree_toggle_opt = Rc::clone(&master_tree);
    let selector_expanded_clone = Rc::clone(&selector_expanded_groups);
    window.on_toggle_group_option(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let mut set = selector_expanded_clone.borrow_mut();
            let id_str = id.to_string();
            if set.contains(&id_str) {
                set.remove(&id_str);
            } else {
                set.insert(id_str);
            }
            let tree = master_tree_toggle_opt.borrow();
            let next_options = build_group_options(&tree, &set);
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_options))));
        }
    });

    // 绑定树形结构分组折叠 / 展开回调
    let window_weak = window.as_weak();
    let master_tree_toggle = Rc::clone(&master_tree);
    let expanded_clone = Rc::clone(&expanded_groups);
    let search_query_toggle = Rc::clone(&search_query);
    let core_state_toggle = Rc::clone(&core_state);
    window.on_toggle_tree_group(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let mut set = expanded_clone.borrow_mut();
            let id_str = id.to_string();
            let is_expanding = !set.contains(&id_str);
            if set.contains(&id_str) {
                set.remove(&id_str);
            } else {
                set.insert(id_str.clone());
            }
            // 同步至存储层
            let _ = core_state_toggle.storage().groups().set_expanded(&id_str, is_expanding);

            let tree = master_tree_toggle.borrow();
            let q = search_query_toggle.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &set)
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            let gname = tree.iter().find(|n| n.id == id_str).map(|n| n.name.as_str()).unwrap_or(id_str.as_str());
            tracing::debug!(target: "smagical_ui::tree", "{}分组: {} (已同步存储层)", if is_expanding { "展开" } else { "折叠" }, gname);
            sync_ui_debug_logs(&w);
        }
    });

    // 绑定拖拽调序与移动节点回调 (支持树形层级迁移与列表纯展示调序双独立机制)
    let window_weak = window.as_weak();
    let master_tree_move = Rc::clone(&master_tree);
    let master_cards_move = Rc::clone(&master_cards);
    let expanded_move = Rc::clone(&expanded_groups);
    let selector_expanded_move = Rc::clone(&selector_expanded_groups);
    let search_query_move = Rc::clone(&search_query);
    let core_state_move = Rc::clone(&core_state);
    window.on_move_tree_node(move |src_id, target_id, drop_position| {
        if let Some(w) = window_weak.upgrade() {
            let src_str = src_id.to_string();
            let target_str = target_id.to_string();
            let pos_str = drop_position.to_string();
            let view_mode = w.get_hosts_view_mode().to_string();

            // 1. 卡片平铺列表模式 (Card View Mode): 纯视觉显示排序调整，绝对锁定所属分组 (parent_id/group) 不变
            if view_mode == "card" {
                let mut cards = master_cards_move.borrow_mut();
                if let (Some(src_idx), Some(tgt_idx)) = (
                    cards.iter().position(|c| c.id == src_str.as_str()),
                    cards.iter().position(|c| c.id == target_str.as_str()),
                )
                    && src_idx != tgt_idx
                {
                    let item = cards.remove(src_idx);
                    let target_insert_idx = if pos_str == "before" {
                        if src_idx < tgt_idx { tgt_idx.saturating_sub(1) } else { tgt_idx }
                    } else {
                        if src_idx < tgt_idx { tgt_idx } else { tgt_idx + 1 }
                    };
                    let final_pos = target_insert_idx.min(cards.len());
                    let item_name = item.name.to_string();
                    let tgt_name = cards.get(tgt_idx.min(cards.len().saturating_sub(1))).map(|c| c.name.to_string()).unwrap_or_default();
                    cards.insert(final_pos, item);

                    // 同步列表排序至存储层
                    let ordered_ids: Vec<String> = cards.iter().map(|c| c.id.to_string()).collect();
                    let _ = core_state_move.storage().hosts().update_list_order(&ordered_ids);

                    let q = search_query_move.borrow().clone();
                    let display_cards: Vec<HostItemData> = if q.is_empty() {
                        cards.clone()
                    } else {
                        cards.iter().filter(|h| {
                            h.name.to_lowercase().contains(&q)
                                || h.address.to_lowercase().contains(&q)
                                || h.group.to_lowercase().contains(&q)
                        }).cloned().collect()
                    };
                    w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(display_cards))));

                    tracing::info!(target: "smagical_ui::hosts", "成功调整列表模式主机展示顺序: [{}] 排在 [{}] 之后 (分组保持锁定，已同步存储层)", item_name, tgt_name);
                    sync_ui_debug_logs(&w);
                }
                return;
            }


            // 2. 树形层级模式 (Tree View Mode): 物理资产层级结构与文件夹迁移
            let mut tree = master_tree_move.borrow_mut();

            match move_and_reorder_raw_node(&mut tree, &src_str, &target_str, &pos_str) {
                Ok((src_name, target_name)) => {
                    // 如果移动到了具体分组内部，自动将该目标分组及其祖先加入展开集合
                    let mut exp = expanded_move.borrow_mut();
                    if pos_str == "inside" && !target_str.is_empty() {
                        let mut curr = target_str.clone();
                        while !curr.is_empty() {
                            exp.insert(curr.clone());
                            if let Some(p) = tree.iter().find(|n| n.id == curr) {
                                curr = p.parent_id.clone();
                            } else {
                                break;
                            }
                        }
                    }

                    // 刷新树形视图与选择器选项
                    let q = search_query_move.borrow().clone();
                    let next_nodes = if q.is_empty() {
                        build_visible_tree_nodes(&tree, &exp)
                    } else {
                        build_search_tree_nodes(&tree, &q)
                    };
                    w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
                    w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

                    let next_options = build_group_options(&tree, &selector_expanded_move.borrow());
                    w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_options))));

                    // 同步树形结构迁移至存储层 (Host or Group)
                    if let Some(moved_node) = tree.iter().find(|n| n.id == src_str) {
                        if moved_node.is_group {
                            let _ = core_state_move.storage().groups().move_group(
                                &src_str,
                                if moved_node.parent_id.is_empty() { None } else { Some(&moved_node.parent_id) },
                            );
                        } else if let Some(mut host_rec) = core_state_move.storage().hosts().get_by_id(&src_str).ok().flatten() {
                            host_rec.parent_group_id = if moved_node.parent_id.is_empty() { None } else { Some(moved_node.parent_id.clone()) };
                            let _ = core_state_move.storage().hosts().save(&host_rec);
                        }
                    }

                    // 树形模式下移动了主机：同步更新列表模式中的所属分组徽章，同时保留用户在列表模式下的自定义相对排序
                    let new_group_name = if let Some(n) = tree.iter().find(|item| item.id == src_str) {
                        if !n.parent_id.is_empty() {
                            tree.iter().find(|item| item.id == n.parent_id).map(|item| item.name.clone()).unwrap_or_else(|| "未分组".to_string())
                        } else {
                            "未分组".to_string()
                        }
                    } else {
                        "未分组".to_string()
                    };

                    let mut cards = master_cards_move.borrow_mut();
                    for card in cards.iter_mut() {
                        if card.id == src_str.as_str() {
                            card.group = new_group_name.clone().into();
                        }
                    }

                    let display_cards: Vec<HostItemData> = if q.is_empty() {
                        cards.clone()
                    } else {
                        cards.iter().filter(|h| {
                            h.name.to_lowercase().contains(&q)
                                || h.address.to_lowercase().contains(&q)
                                || h.group.to_lowercase().contains(&q)
                        }).cloned().collect()
                    };
                    w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(display_cards))));

                    tracing::info!(target: "smagical_ui::hosts", "成功调序/移动树节点 [{}] (模式: {}, 目标: [{}], 已同步存储层)", src_name, pos_str, target_name);
                    sync_ui_debug_logs(&w);
                }
                Err(err_msg) => {
                    tracing::warn!(target: "smagical_ui::hosts", "调序/移动树节点失败: {}", err_msg);
                    sync_ui_debug_logs(&w);
                }
            }
        }
    });

    // 绑定实时拖拽悬停落点计算回调 (支持树形层级与卡片平铺双模式)
    let window_weak = window.as_weak();
    let master_tree_hover = Rc::clone(&master_tree);
    window.on_request_drag_hover(move |src_id, target_idx, _offset_in_row| {
        if let Some(w) = window_weak.upgrade() {
            let src_str = src_id.to_string();
            let view_mode = w.get_hosts_view_mode().to_string();

            if target_idx < 0 {
                w.set_drop_target_index(-1);
                w.set_drop_target_id("root".into());
                w.set_drop_target_name("顶级根目录 (未分组)".into());
                w.set_drop_position("root".into());
                w.set_drop_target_valid(true);
                return;
            }

            // 1. 卡片平铺列表模式 (Card View Mode)
            if view_mode == "card" {
                let current_hosts = w.get_hosts();
                let total_len = current_hosts.row_count();
                if (target_idx as usize) < total_len {
                    if let Some(target) = current_hosts.row_data(target_idx as usize) {
                        let target_id = target.id.to_string();
                        let target_name = target.name.to_string();
                        let is_valid = target_id != src_str;

                        w.set_drop_target_index(target_idx);
                        w.set_drop_target_id(target_id.into());
                        w.set_drop_target_name(target_name.into());
                        w.set_drop_position("after".into());
                        w.set_drop_target_valid(is_valid);
                    }
                } else {
                    w.set_drop_target_index(-1);
                    w.set_drop_target_id("".into());
                    w.set_drop_target_name("".into());
                    w.set_drop_position("".into());
                    w.set_drop_target_valid(false);
                }
                return;
            }

            // 2. 树形层级模式 (Tree View Mode)
            let visible_nodes = w.get_tree_nodes();
            let total_len = visible_nodes.row_count();

            if (target_idx as usize) < total_len {
                if let Some(target) = visible_nodes.row_data(target_idx as usize) {
                    let is_target_group = target.is_group;
                    let target_id = target.id.to_string();
                    let target_name = target.name.to_string();

                    // 核心规则：
                    // 1. 拖到文件夹（或文件夹下线） -> 移入该文件夹内部 ("inside")
                    // 2. 拖到文件夹下的主机（或主机下线） -> 排在该主机的下面 ("after")
                    let position = if is_target_group {
                        "inside"
                    } else {
                        "after"
                    };

                    // 循环引用与自身校验 (读取 master_tree)
                    let tree = master_tree_hover.borrow();
                    let mut is_valid = true;
                    if target_id == src_str {
                        is_valid = false;
                    } else {
                        let is_src_group = tree.iter().find(|n| n.id == src_str).map(|n| n.is_group).unwrap_or(false);
                        if is_src_group {
                            let mut curr = if position == "inside" {
                                target_id.clone()
                            } else {
                                target.parent_id.to_string()
                            };
                            while !curr.is_empty() {
                                if curr == src_str {
                                    is_valid = false;
                                    break;
                                }
                                if let Some(pn) = tree.iter().find(|n| n.id == curr) {
                                    curr = pn.parent_id.clone();
                                } else {
                                    break;
                                }
                            }
                        }
                    }

                    w.set_drop_target_index(target_idx);
                    w.set_drop_target_id(target_id.into());
                    w.set_drop_target_name(target_name.into());
                    w.set_drop_position(position.into());
                    w.set_drop_target_valid(is_valid);
                }
            } else {
                w.set_drop_target_index(-1);
                w.set_drop_target_id("".into());
                w.set_drop_target_name("".into());
                w.set_drop_position("".into());
                w.set_drop_target_valid(false);
            }
        }
    });

    // 绑定新建分组回调 (支持树状层级指定上级与即时展开)
    let window_weak = window.as_weak();
    let master_tree_create = Rc::clone(&master_tree);
    let expanded_create = Rc::clone(&expanded_groups);
    let selector_expanded_create = Rc::clone(&selector_expanded_groups);
    let search_query_create = Rc::clone(&search_query);
    let next_gid_create = Rc::clone(&next_group_id);
    let core_state_create = Rc::clone(&core_state);
    window.on_create_group(move |parent_id, name| {
        if let Some(w) = window_weak.upgrade() {
            let p_id = parent_id.to_string();
            let g_name = name.trim().to_string();
            if g_name.is_empty() {
                return;
            }

            let mut tree = master_tree_create.borrow_mut();
            let mut gid_counter = next_gid_create.borrow_mut();
            *gid_counter += 1;
            let new_id = format!("grp-custom-{}", *gid_counter);

            let (target_parent_id, level) = if p_id == "root" || p_id.is_empty() {
                ("".to_string(), 0)
            } else {
                let parent_level = tree.iter().find(|n| n.id == p_id).map(|n| n.level).unwrap_or(0);
                (p_id.clone(), parent_level + 1)
            };

            let new_group_node = RawTreeNode {
                id: new_id.clone(),
                name: g_name.clone(),
                is_group: true,
                parent_id: target_parent_id.clone(),
                level,
                address: "".to_string(),
                port: 0,
                status: "online".to_string(),
                ping_ms: 0,
                item_count: 0,
            };

            // 同步保存至存储层
            let group_rec = if target_parent_id.is_empty() {
                GroupRecord::root(new_id.clone(), g_name.clone())
            } else {
                GroupRecord::child(new_id.clone(), g_name.clone(), target_parent_id.clone(), level)
            };
            let _ = core_state_create.storage().groups().save(&group_rec);

            // 智能定位插入位置：插入到同父节点的子项末尾，或追加到分组后
            let mut insert_pos = tree.len();
            if !target_parent_id.is_empty() {
                let mut last_child_idx = None;
                for (idx, node) in tree.iter().enumerate() {
                    if node.id == target_parent_id || node.parent_id == target_parent_id {
                        last_child_idx = Some(idx);
                    }
                }
                if let Some(idx) = last_child_idx {
                    insert_pos = idx + 1;
                }
                // 确保父节点处于展开状态，以便立刻看见新建的分组
                expanded_create.borrow_mut().insert(target_parent_id.clone());
                selector_expanded_create.borrow_mut().insert(target_parent_id);
            }
            // 新创建的分组自身默认展开
            expanded_create.borrow_mut().insert(new_id);

            tree.insert(insert_pos, new_group_node);

            // 刷新弹窗中的上级分组列表选项
            let next_options = build_group_options(&tree, &selector_expanded_create.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_options))));

            // 刷新主界面树形结构
            let q = search_query_create.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_create.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_ui::tree", "创建新分组: {} (上级: {}, 已同步存储层)", g_name, if p_id.is_empty() { "根目录" } else { &p_id });
            sync_ui_debug_logs(&w);
        }
    });

    // 绑定主机实时搜索过滤回调 (双向联动树形视图与卡片列表，保持自定义排序)
    let window_weak = window.as_weak();
    let master_tree_filter = Rc::clone(&master_tree);
    let master_cards_filter = Rc::clone(&master_cards);
    let expanded_clone = Rc::clone(&expanded_groups);
    let search_query_filter = Rc::clone(&search_query);
    window.on_filter_hosts(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let q = query.trim().to_lowercase();
            *search_query_filter.borrow_mut() = q.clone();

            // 1. 动态过滤树形节点
            let tree = master_tree_filter.borrow();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_clone.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            // 2. 动态过滤卡片列表 (基于当前 master_cards 列表及用户自定义排序)
            let cards = master_cards_filter.borrow();
            let filtered_cards: Vec<HostItemData> = cards
                .iter()
                .filter(|h| {
                    if q.is_empty() {
                        true
                    } else {
                        h.name.to_lowercase().contains(&q)
                            || h.address.to_lowercase().contains(&q)
                            || h.group.to_lowercase().contains(&q)
                    }
                })
                .cloned()
                .collect();
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(filtered_cards))));

            if !q.is_empty() {
                tracing::debug!(target: "smagical_ui::search", "过滤主机资产: '{}'", q);
                sync_ui_debug_logs(&w);
            }
        }
    });

    // 绑定主机打开回调 (从左侧主机列表双击时打开或多开对应的终端 Tab)
    let window_weak = window.as_weak();
    let master_tree_open = Rc::clone(&master_tree);
    let active_sessions_open = Rc::clone(&active_sessions);
    let next_session_num_open = Rc::clone(&next_session_num);
    let cached_shells_open = Rc::clone(&cached_shells);
    window.on_open_host(move |host_id| {
        if let Some(w) = window_weak.upgrade() {
            let h_id = host_id.to_string();

            // 支持启动本地终端环境 (使用缓存的 Shell 列表，避免重复扫描文件系统)
            if h_id.starts_with("local-") {
                let mut sessions = active_sessions_open.borrow_mut();
                let mut num = next_session_num_open.borrow_mut();

                let sess_id = format!("sess-{}", *num);
                *num += 1;

                let all_shells = &*cached_shells_open;
                let (base_name, addr) = if let Some(sh) = all_shells.iter().find(|s| s.id == h_id.as_str()) {
                    (sh.title.to_string(), format!("Local ({})", sh.subtitle))
                } else {
                    let fallback_name = match h_id.as_str() {
                        "local-pwsh7" => "PowerShell 7",
                        "local-powershell" => "PowerShell",
                        "local-wsl" => "WSL (Linux)",
                        "local-cmd" => "Command Prompt",
                        "local-gitbash" => "Git Bash",
                        "local-bash" => "Bash",
                        "local-zsh" => "Zsh",
                        "local-fish" => "Fish",
                        "local-sh" => "Sh",
                        "local-nushell" => "Nushell",
                        _ => "Local Shell",
                    };
                    (fallback_name.to_string(), "Local Terminal".to_string())
                };

                let count = sessions.iter().filter(|s| s.host_id == h_id).count();
                let display_title = if count == 0 {
                    base_name.clone()
                } else {
                    format!("{} ({})", base_name, count + 1)
                };

                let new_sess = TerminalSessionInfo {
                    session_id: sess_id.clone(),
                    host_id: h_id.clone(),
                    host_name: base_name,
                    host_address: addr,
                    host_status: "online".to_string(),
                    ping_ms: 0,
                    display_title: display_title.clone(),
                };

                sessions.push(new_sess);
                sync_active_session_ui(&w, &sessions, &sess_id);

                tracing::info!(target: "smagical_ui::session", "启动本地终端环境: {} -> Session ID: {}", display_title, sess_id);
                sync_ui_debug_logs(&w);
                return;
            }

            let tree = master_tree_open.borrow();

            // 查找目标主机节点
            if let Some(node) = tree.iter().find(|n| n.id == h_id && !n.is_group) {
                let mut sessions = active_sessions_open.borrow_mut();
                let mut num = next_session_num_open.borrow_mut();

                let sess_id = format!("sess-{}", *num);
                *num += 1;

                // 计算该主机已有多少个活跃会话 (用于智能多开编号: name, name (2), name (3)...)
                let count = sessions.iter().filter(|s| s.host_id == h_id).count();
                let display_title = if count == 0 {
                    node.name.clone()
                } else {
                    format!("{} ({})", node.name, count + 1)
                };

                let addr = if node.address.is_empty() {
                    "127.0.0.1:22".to_string()
                } else if node.port > 0 {
                    format!("{}:{}", node.address, node.port)
                } else {
                    node.address.clone()
                };

                let new_sess = TerminalSessionInfo {
                    session_id: sess_id.clone(),
                    host_id: h_id.clone(),
                    host_name: node.name.clone(),
                    host_address: addr,
                    host_status: node.status.clone(),
                    ping_ms: node.ping_ms,
                    display_title: display_title.clone(),
                };

                sessions.push(new_sess);
                sync_active_session_ui(&w, &sessions, &sess_id);

                tracing::info!(target: "smagical_ui::session", "发起远程终端连接: {} -> Session ID: {}", display_title, sess_id);
                sync_ui_debug_logs(&w);
            }
        }
    });

    // 绑定新建 Tab 回调 (点击 Tab 栏 + 号时打开快速新建终端会话中心居中弹窗)
    let window_weak = window.as_weak();
    let master_tree_reset = Rc::clone(&master_tree);
    let cached_shells_new_tab = Rc::clone(&cached_shells);
    window.on_new_tab(move || {
        if let Some(w) = window_weak.upgrade() {
            // 重置搜索框与弹窗列表 (使用启动时缓存的本地终端列表)
            w.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from((*cached_shells_new_tab).clone()))));


            let tree = master_tree_reset.borrow();
            let all_hosts: Vec<HostItemData> = tree
                .iter()
                .filter(|n| !n.is_group)
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
            w.set_launcher_host_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(all_hosts))));

            w.set_is_new_session_modal_open(true);
        }
    });

    // 绑定新建终端会话弹窗实时搜索过滤回调
    let window_weak = window.as_weak();
    let master_tree_launcher = Rc::clone(&master_tree);
    let cached_shells_launcher = Rc::clone(&cached_shells);
    window.on_filter_launcher(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let q = query.trim().to_lowercase();

            // [P2 Fix] 使用缓存的 Shell 列表过滤，避免每次按键都重扫文件系统
            let filtered_locals: Vec<LocalShellItemData> = if q.is_empty() {
                (*cached_shells_launcher).clone()
            } else {
                cached_shells_launcher
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
                .filter(|n| !n.is_group)
                .filter(|n| {
                    if q.is_empty() {
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

    // 绑定快捷命令发送回调
    let window_weak = window.as_weak();
    window.on_send_snippet(move |cmd| {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::cmd", "向终端发送指令片段: {}", cmd);
            sync_ui_debug_logs(&w);
        }
    });

    // =========================================================================
    // 开发者调试控制面板与批量生成事件绑定 (Debug Workbench Handlers)
    // =========================================================================

    // 0.1 批量生成主机资产
    let window_weak = window.as_weak();
    let master_tree_bg = Rc::clone(&master_tree);
    let expanded_bg = Rc::clone(&expanded_groups);
    let selector_bg = Rc::clone(&selector_expanded_groups);
    let search_bg = Rc::clone(&search_query);
    window.on_debug_batch_generate(move |prefix, count_str, ip_prefix, start_ip_str, port_str, group, status_mode, overwrite| {
        if let Some(w) = window_weak.upgrade() {
            let p_str = prefix.to_string();
            let ip_p_str = ip_prefix.to_string();
            let grp_str = group.to_string();
            let st_str = status_mode.to_string();
            let cnt = count_str.as_str().parse::<usize>().unwrap_or(10);
            let start_ip = start_ip_str.as_str().parse::<usize>().unwrap_or(10);
            let port = port_str.as_str().parse::<i32>().unwrap_or(22);

            let config = BatchGenerateConfig {
                name_prefix: if p_str.is_empty() { "node-".to_string() } else { p_str },
                count: if cnt == 0 { 10 } else { cnt },
                start_index: 1,
                ip_prefix: if ip_p_str.is_empty() { "192.168.1.".to_string() } else { ip_p_str },
                start_ip,
                port,
                group_name: if grp_str.is_empty() { "批量集群".to_string() } else { grp_str.clone() },
                status_mode: st_str,
            };

            let (new_tree_raw, new_cards_raw) = generate_batch_hosts(&config);
            let new_tree: Vec<RawTreeNode> = new_tree_raw.into_iter().map(RawTreeNode::from).collect();
            let new_cards: Vec<HostItemData> = new_cards_raw
                .into_iter()
                .map(|c| HostItemData {
                    id: c.id.into(),
                    name: c.name.into(),
                    address: c.address.into(),
                    port: c.port,
                    group: c.group.into(),
                    status: c.status.into(),
                    ping_ms: c.ping_ms,
                })
                .collect();

            if overwrite {
                *master_tree_bg.borrow_mut() = new_tree.clone();
                w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(new_cards))));
            } else {
                let mut current_tree = master_tree_bg.borrow_mut();
                let (leaf_gid, leaf_lvl, _leaf_name) = ensure_raw_group_hierarchy(&mut current_tree, &grp_str);
                
                // 将新生成的 host 节点挂入已存在/新建的叶子分组
                for n in &new_tree {
                    if !n.is_group {
                        let mut host_node = n.clone();
                        host_node.parent_id = leaf_gid.clone();
                        host_node.level = leaf_lvl + 1;
                        current_tree.push(host_node);
                    }
                }

                let hosts = w.get_hosts();
                let mut host_list: Vec<HostItemData> = (0..hosts.row_count()).filter_map(|i| hosts.row_data(i)).collect();
                host_list.extend(new_cards);
                w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(host_list))));
            }

            // 展开新增的分组
            for n in &new_tree {
                if n.is_group {
                    expanded_bg.borrow_mut().insert(n.id.clone());
                    selector_bg.borrow_mut().insert(n.id.clone());
                }
            }

            let tree = master_tree_bg.borrow();
            let opts = build_group_options(&tree, &selector_bg.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let q = search_bg.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_bg.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::batch", "成功批量生成主机: {} 台 (归属: {})", cnt, grp_str);
            sync_ui_debug_logs(&w);
        }
    });

    // 0.2 批量更新主机状态
    let window_weak = window.as_weak();
    let master_tree_bs = Rc::clone(&master_tree);
    let expanded_bs = Rc::clone(&expanded_groups);
    let search_bs = Rc::clone(&search_query);
    let core_state_bs = Rc::clone(&core_state);
    window.on_debug_batch_update_status(move |status_mode| {
        if let Some(w) = window_weak.upgrade() {
            let st = status_mode.as_str();
            let mut tree = master_tree_bs.borrow_mut();
            for (i, node) in tree.iter_mut().enumerate() {
                if !node.is_group {
                    let (s, ping) = match st {
                        "all_online" | "online" => ("online", 18),
                        "all_offline" | "offline" => ("offline", 0),
                        "all_warning" | "warning" => ("warning", 160),
                        _ => {
                            if i % 3 == 0 {
                                ("warning", 135)
                            } else if i % 4 == 0 {
                                ("offline", 0)
                            } else {
                                ("online", 20)
                            }
                        }
                    };
                    node.status = s.to_string();
                    node.ping_ms = ping;
                }
            }

            let hosts = w.get_hosts();
            let mut host_list: Vec<HostItemData> = (0..hosts.row_count()).filter_map(|i| hosts.row_data(i)).collect();
            for (i, card) in host_list.iter_mut().enumerate() {
                let (s, ping) = match st {
                    "all_online" | "online" => ("online", 18),
                    "all_offline" | "offline" => ("offline", 0),
                    "all_warning" | "warning" => ("warning", 160),
                    _ => {
                        if i % 3 == 0 {
                            ("warning", 135)
                        } else if i % 4 == 0 {
                            ("offline", 0)
                        } else {
                            ("online", 20)
                        }
                    }
                };
                card.status = s.into();
                card.ping_ms = ping;
            }
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(host_list))));

            // [P1 Fix] 同步批量状态更新至存储层
            if let Ok(stored_hosts) = core_state_bs.storage().hosts().list_all() {
                let updated: Vec<smagical_core::HostRecord> = stored_hosts.into_iter().map(|mut h| {
                    let new_status = match st {
                        "all_online" | "online" => smagical_core::HostStatus::Online,
                        "all_offline" | "offline" => smagical_core::HostStatus::Offline,
                        "all_warning" | "warning" => smagical_core::HostStatus::Warning,
                        _ => smagical_core::HostStatus::Online,
                    };
                    h.status = new_status;
                    h
                }).collect();
                let _ = core_state_bs.storage().hosts().save_batch(&updated);
            }

            let q = search_bs.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_bs.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::batch", "批量变更全量主机状态为: {}", st);
            sync_ui_debug_logs(&w);
        }
    });


    // 0.3 批量更新 SSH 端口
    let window_weak = window.as_weak();
    let master_tree_bp = Rc::clone(&master_tree);
    let expanded_bp = Rc::clone(&expanded_groups);
    let search_bp = Rc::clone(&search_query);
    window.on_debug_batch_update_port(move |new_port_str| {
        if let Some(w) = window_weak.upgrade() {
            let new_port = new_port_str.as_str().parse::<i32>().unwrap_or(22);
            let mut tree = master_tree_bp.borrow_mut();
            for node in tree.iter_mut() {
                if !node.is_group {
                    node.port = new_port;
                }
            }

            let hosts = w.get_hosts();
            let mut host_list: Vec<HostItemData> = (0..hosts.row_count()).filter_map(|i| hosts.row_data(i)).collect();
            for card in host_list.iter_mut() {
                card.port = new_port;
            }
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(host_list))));

            let q = search_bp.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_bp.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::batch", "批量更新全量主机 SSH 端口为: {}", new_port);
            sync_ui_debug_logs(&w);
        }
    });

    // 1. 场景预设一键注入
    let window_weak = window.as_weak();
    let master_tree_dbg = Rc::clone(&master_tree);
    let expanded_dbg = Rc::clone(&expanded_groups);
    let selector_dbg = Rc::clone(&selector_expanded_groups);
    let search_dbg = Rc::clone(&search_query);
    window.on_debug_inject_preset(move |preset_id| {
        if let Some(w) = window_weak.upgrade() {
            let pid = preset_id.as_str();
            let (new_tree_raw, new_cards_raw) = get_preset_by_id(pid);
            let new_tree: Vec<RawTreeNode> = new_tree_raw.into_iter().map(RawTreeNode::from).collect();
            let new_cards: Vec<HostItemData> = new_cards_raw
                .into_iter()
                .map(|c| HostItemData {
                    id: c.id.into(),
                    name: c.name.into(),
                    address: c.address.into(),
                    port: c.port,
                    group: c.group.into(),
                    status: c.status.into(),
                    ping_ms: c.ping_ms,
                })
                .collect();

            *master_tree_dbg.borrow_mut() = new_tree.clone();

            // 重置展开状态（默认展开所有顶级分组）
            let mut new_exp = HashSet::new();
            for n in &new_tree {
                if n.is_group {
                    new_exp.insert(n.id.clone());
                }
            }
            *expanded_dbg.borrow_mut() = new_exp.clone();
            new_exp.insert("root".to_string());
            *selector_dbg.borrow_mut() = new_exp.clone();

            // 刷新弹窗上级分组选项
            let opts = build_group_options(&new_tree, &selector_dbg.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            // 刷新树形节点与视口计算宽度
            let q = search_dbg.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&new_tree, &expanded_dbg.borrow())
            } else {
                build_search_tree_nodes(&new_tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            // 刷新卡片列表
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(new_cards))));

            tracing::info!(target: "smagical_debug::preset", "成功注入场景预设: {}", pid);
            sync_ui_debug_logs(&w);
        }
    });

    // 2. 快速新增主机 (支持路径嵌套，例如: 集群/k8s)
    let window_weak = window.as_weak();
    let master_tree_qh = Rc::clone(&master_tree);
    let expanded_qh = Rc::clone(&expanded_groups);
    let selector_qh = Rc::clone(&selector_expanded_groups);
    let search_qh = Rc::clone(&search_query);
    let core_state_qh = Rc::clone(&core_state);
    let next_hid = Rc::new(RefCell::new(100));
    window.on_debug_quick_add_host(move |name, ip, port_str, group| {
        if let Some(w) = window_weak.upgrade() {
            let h_name = name.trim().to_string();
            let h_ip = ip.trim().to_string();
            let h_grp = group.trim().to_string();
            let port = port_str.as_str().parse::<i32>().unwrap_or(22);
            if h_name.is_empty() { return; }

            let mut counter = next_hid.borrow_mut();
            *counter += 1;
            let new_id = format!("custom-host-{}", *counter);

            let mut tree = master_tree_qh.borrow_mut();

            let (parent_id, level, display_grp) = if !h_grp.is_empty() {
                let (pid, lvl, name) = ensure_raw_group_hierarchy(&mut tree, &h_grp);
                for n in tree.iter() {
                    if n.is_group {
                        expanded_qh.borrow_mut().insert(n.id.clone());
                        selector_qh.borrow_mut().insert(n.id.clone());
                    }
                }
                (pid, lvl + 1, name)
            } else {
                ("".to_string(), 0, "未分组".to_string())
            };

            let node = RawTreeNode {
                id: new_id.clone(),
                name: h_name.clone(),
                is_group: false,
                parent_id: parent_id.clone(),
                level,
                address: h_ip.clone(),
                port,
                status: "online".to_string(),
                ping_ms: 22,
                item_count: 0,
            };

            tree.push(node);

            // [P1 Fix] 同步新增主机至存储层，避免与 storage 双真相来源分叉
            let host_rec = smagical_core::HostRecord {
                id: new_id.clone(),
                name: h_name.clone(),
                address: h_ip.clone(),
                port: port as u16,
                parent_group_id: if parent_id.is_empty() { None } else { Some(parent_id) },
                status: smagical_core::HostStatus::Online,
                ping_ms: 22,
                sort_order: 0,
                notes: String::new(),
            };
            let _ = core_state_qh.storage().hosts().save(&host_rec);

            let opts = build_group_options(&tree, &selector_qh.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let q = search_qh.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_qh.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            let card = HostItemData {
                id: new_id.into(),
                name: h_name.clone().into(),
                address: h_ip.clone().into(),
                port,
                group: display_grp.into(),
                status: "online".into(),
                ping_ms: 22,
            };
            let hosts = w.get_hosts();
            let mut host_list: Vec<HostItemData> = (0..hosts.row_count()).filter_map(|i| hosts.row_data(i)).collect();
            host_list.push(card);
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(host_list))));

            tracing::info!(target: "smagical_debug::data", "快速添加主机: {} ({}:{})", h_name, h_ip, port);
            sync_ui_debug_logs(&w);
        }
    });


    // 3. 快速新增分组 (支持路径嵌套，例如: 集群/k8s)
    let window_weak = window.as_weak();
    let master_tree_qg = Rc::clone(&master_tree);
    let expanded_qg = Rc::clone(&expanded_groups);
    let selector_qg = Rc::clone(&selector_expanded_groups);
    let search_qg = Rc::clone(&search_query);
    window.on_debug_quick_add_group(move |name, _parent| {
        if let Some(w) = window_weak.upgrade() {
            let g_name = name.trim().to_string();
            if g_name.is_empty() { return; }

            let mut tree = master_tree_qg.borrow_mut();
            ensure_raw_group_hierarchy(&mut tree, &g_name);

            // 展开所有分组
            for n in tree.iter() {
                if n.is_group {
                    expanded_qg.borrow_mut().insert(n.id.clone());
                    selector_qg.borrow_mut().insert(n.id.clone());
                }
            }

            let opts = build_group_options(&tree, &selector_qg.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let q = search_qg.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_qg.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::data", "快速添加分组层级: {}", g_name);
            sync_ui_debug_logs(&w);
        }
    });

    // 4. 清空全量数据
    let window_weak = window.as_weak();
    let master_tree_clr = Rc::clone(&master_tree);
    let core_state_clr = Rc::clone(&core_state);
    window.on_debug_clear_data(move || {
        if let Some(w) = window_weak.upgrade() {
            master_tree_clr.borrow_mut().clear();
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::<crate::generated::HostTreeNode>::new()))));
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::<crate::generated::HostItemData>::new()))));
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::<crate::generated::GroupOptionData>::new()))));
            w.set_tree_content_width(240.0_f32);
            // [P1 Fix] 同步清空存储层
            if let Ok(hosts) = core_state_clr.storage().hosts().list_all() {
                for h in &hosts { let _ = core_state_clr.storage().hosts().delete(&h.id); }
            }
            if let Ok(groups) = core_state_clr.storage().groups().list_all() {
                for g in &groups { let _ = core_state_clr.storage().groups().delete(&g.id); }
            }
            tracing::warn!(target: "smagical_debug::data", "全量主机与分组数据已被清空");
            sync_ui_debug_logs(&w);
        }
    });


    // 5. 恢复默认数据
    let window_weak = window.as_weak();
    let master_tree_rst = Rc::clone(&master_tree);
    let expanded_rst = Rc::clone(&expanded_groups);
    let selector_rst = Rc::clone(&selector_expanded_groups);
    window.on_debug_reset_default_data(move || {
        if let Some(w) = window_weak.upgrade() {
            let (def_tree_raw, def_cards_raw) = get_preset_by_id("minimal");
            let def_tree: Vec<RawTreeNode> = def_tree_raw.into_iter().map(RawTreeNode::from).collect();
            let def_cards: Vec<HostItemData> = def_cards_raw
                .into_iter()
                .map(|c| HostItemData {
                    id: c.id.into(),
                    name: c.name.into(),
                    address: c.address.into(),
                    port: c.port,
                    group: c.group.into(),
                    status: c.status.into(),
                    ping_ms: c.ping_ms,
                })
                .collect();
            *master_tree_rst.borrow_mut() = def_tree.clone();

            *expanded_rst.borrow_mut() = HashSet::from(["grp-prod".to_string()]);
            *selector_rst.borrow_mut() = HashSet::from(["root".to_string(), "grp-prod".to_string()]);

            let opts = build_group_options(&def_tree, &selector_rst.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let next_nodes = build_visible_tree_nodes(&def_tree, &expanded_rst.borrow());
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(def_cards))));

            tracing::info!(target: "smagical_debug::data", "已重置恢复为默认初始数据");
            sync_ui_debug_logs(&w);
        }
    });

    // 6. 清空日志
    let window_weak = window.as_weak();
    window.on_debug_clear_logs(move || {
        if let Some(w) = window_weak.upgrade() {
            if let Ok(mut buf) = smagical_debug::get_global_log_buffer().lock() {
                buf.clear();
            }
            sync_ui_debug_logs(&w);
        }
    });

    // 7. 模拟生成测试日志
    let window_weak = window.as_weak();
    window.on_debug_emit_test_log(move |_level| {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::test", "这是测试 INFO 日志消息");
            tracing::warn!(target: "smagical_ui::net", "检测到网络延迟波动: 128ms");
            tracing::error!(target: "smagical_ui::ssh", "连接目标主机 host-prod-01 超时");
            sync_ui_debug_logs(&w);
        }
    });

    // 绑定无边框窗口控制回调
    window.on_close_window(|| {
        let _ = slint::quit_event_loop();
    });

    let window_weak = window.as_weak();
    window.on_minimize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            w.window().set_minimized(true);
        }
    });

    let window_weak = window.as_weak();
    window.on_maximize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            let is_max = w.window().is_maximized();
            w.window().set_maximized(!is_max);
            w.set_is_window_maximized(!is_max);
        }
    });

    window.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tree() -> Vec<RawTreeNode> {
        vec![
            RawTreeNode {
                id: "grp-a".into(),
                name: "分组A".into(),
                is_group: true,
                parent_id: "".into(),
                level: 0,
                address: "".into(),
                port: 0,
                status: "online".into(),
                ping_ms: 0,
                item_count: 2,
            },
            RawTreeNode {
                id: "grp-a-sub".into(),
                name: "子分组A1".into(),
                is_group: true,
                parent_id: "grp-a".into(),
                level: 1,
                address: "".into(),
                port: 0,
                status: "online".into(),
                ping_ms: 0,
                item_count: 1,
            },
            RawTreeNode {
                id: "host-1".into(),
                name: "host-01".into(),
                is_group: false,
                parent_id: "grp-a-sub".into(),
                level: 2,
                address: "10.0.0.1".into(),
                port: 22,
                status: "online".into(),
                ping_ms: 10,
                item_count: 0,
            },
            RawTreeNode {
                id: "grp-b".into(),
                name: "分组B".into(),
                is_group: true,
                parent_id: "".into(),
                level: 0,
                address: "".into(),
                port: 0,
                status: "online".into(),
                ping_ms: 0,
                item_count: 0,
            },
            RawTreeNode {
                id: "host-root".into(),
                name: "host-root-node".into(),
                is_group: false,
                parent_id: "".into(),
                level: 0,
                address: "10.0.0.99".into(),
                port: 22,
                status: "online".into(),
                ping_ms: 5,
                item_count: 0,
            },
        ]
    }

    #[test]
    fn test_move_host_inside_group() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "host-1", "grp-b", "inside");
        assert!(res.is_ok());
        let (src_name, target_name) = res.unwrap();
        assert_eq!(src_name, "host-01");
        assert_eq!(target_name, "分组B");

        let host = tree.iter().find(|n| n.id == "host-1").unwrap();
        assert_eq!(host.parent_id, "grp-b");
        assert_eq!(host.level, 1);
    }

    #[test]
    fn test_reorder_before() {
        let mut tree = create_test_tree();
        // 将 grp-b 拖到 grp-a 前面 (Before 调序)
        let res = move_and_reorder_raw_node(&mut tree, "grp-b", "grp-a", "before");
        assert!(res.is_ok());

        assert_eq!(tree[0].id, "grp-b");
        assert_eq!(tree[1].id, "grp-a");
    }

    #[test]
    fn test_reorder_after() {
        let mut tree = create_test_tree();
        // 将 host-root 拖到 grp-a 后面 (After 调序)
        let res = move_and_reorder_raw_node(&mut tree, "host-root", "grp-a", "after");
        assert!(res.is_ok());

        // grp-a 包含其子树 (grp-a, grp-a-sub, host-1)，host-root 应位于子树后面
        let host_pos = tree.iter().position(|n| n.id == "host-root").unwrap();
        let host1_pos = tree.iter().position(|n| n.id == "host-1").unwrap();
        assert!(host_pos > host1_pos);
    }

    #[test]
    fn test_move_host_to_root() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "host-1", "root", "root");
        assert!(res.is_ok());

        let host = tree.iter().find(|n| n.id == "host-1").unwrap();
        assert_eq!(host.parent_id, "");
        assert_eq!(host.level, 0);
    }

    #[test]
    fn test_move_group_with_children() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "grp-a-sub", "grp-b", "inside");
        assert!(res.is_ok());

        let sub = tree.iter().find(|n| n.id == "grp-a-sub").unwrap();
        assert_eq!(sub.parent_id, "grp-b");
        assert_eq!(sub.level, 1);

        let host = tree.iter().find(|n| n.id == "host-1").unwrap();
        assert_eq!(host.parent_id, "grp-a-sub");
        assert_eq!(host.level, 2);
    }

    #[test]
    fn test_prevent_cycle_moving_parent_to_child() {
        let mut tree = create_test_tree();
        // grp-a 是 grp-a-sub 的父级，不能将 grp-a 移入 grp-a-sub
        let res = move_and_reorder_raw_node(&mut tree, "grp-a", "grp-a-sub", "inside");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("循环引用"));
    }

    #[test]
    fn test_cannot_move_inside_self() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "grp-a", "grp-a", "inside");
        assert!(res.is_err());
    }
}
