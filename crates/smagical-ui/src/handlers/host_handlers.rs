//! 主机与分组资产管理、树形/列表视图拖拽移动、快速打开终端等交互回调绑定。
//!
//! 包含多级分组嵌套维护、无环拓扑防呆校验、平滑调序与动态视口宽度计算。

use std::rc::Rc;
use slint::ComponentHandle;
use smagical_core::event::{
    HostGroupToggledEvent, HostSearchFilteredEvent, HostTreeReorderedEvent, TerminalSessionEvent,
};
use smagical_core::GroupRecord;

use crate::generated::{AppWindow, HostItemData};
use crate::handlers::AppContext;
use crate::session::{sync_active_session_ui, TerminalSessionInfo};
use crate::terminal::TerminalInstance;
use crate::tree_model::{
    build_group_options, build_search_tree_nodes, build_visible_tree_nodes,
    calculate_max_tree_width, move_and_reorder_raw_node, RawTreeNode,
};


/// 注册主机资产管理相关交互回调。
///
/// 绑定分组折叠/展开、跨分组拖拽迁移、弹窗选择器级联、实时搜索过滤及双击打开终端等事件。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用
/// - `ctx`: 全局应用共享上下文对象引用
pub(crate) fn register_host_handlers(window: &AppWindow, ctx: &AppContext) {
    // -------------------------------------------------------------------------
    // 1. 新建/编辑主机弹窗中“上级分组选择器”折叠 / 展开回调
    // -------------------------------------------------------------------------
    // 支持在新建主机或新建分组弹窗的下拉树形选择框内收缩或展开某个父级节点。
    let window_weak = window.as_weak();
    let master_tree_toggle_opt = Rc::clone(&ctx.master_tree);
    let selector_expanded_clone = Rc::clone(&ctx.selector_expanded_groups);
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

    // -------------------------------------------------------------------------
    // 2. 侧边栏树形结构分组折叠 / 展开回调
    // -------------------------------------------------------------------------
    // 点击左侧主机树中的某个文件夹节点时触发，切换展开状态并同步持久化至 AppStorage。
    let window_weak = window.as_weak();
    let master_tree_toggle = Rc::clone(&ctx.master_tree);
    let expanded_clone = Rc::clone(&ctx.expanded_groups);
    let search_query_toggle = Rc::clone(&ctx.search_query);
    let core_state_toggle = Rc::clone(&ctx.core_state);
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
            core_state_toggle.events().dispatch(&HostGroupToggledEvent {
                group_id: id_str.clone(),
                is_expanded: is_expanding,
            });


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
        }
    });

    // -------------------------------------------------------------------------
    // 3. 节点移动 / 拖拽层级调序回调
    // -------------------------------------------------------------------------
    // 鼠标拖拽松开后触发：支持树形层级物理迁移与卡片列表视觉调序双模式。
    let window_weak = window.as_weak();
    let master_tree_move = Rc::clone(&ctx.master_tree);
    let master_cards_move = Rc::clone(&ctx.master_cards);
    let expanded_move = Rc::clone(&ctx.expanded_groups);
    let selector_expanded_move = Rc::clone(&ctx.selector_expanded_groups);
    let search_query_move = Rc::clone(&ctx.search_query);
    let core_state_move = Rc::clone(&ctx.core_state);
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
                    core_state_move.events().dispatch(&HostTreeReorderedEvent {
                        source_id: src_str.clone(),
                        target_id: target_str.clone(),
                        position: pos_str.clone(),
                    });
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
                    core_state_move.events().dispatch(&HostTreeReorderedEvent {
                        source_id: src_str.clone(),
                        target_id: target_str.clone(),
                        position: pos_str.clone(),
                    });
                }

                Err(err_msg) => {
                    tracing::warn!(target: "smagical_ui::hosts", "移动节点被阻止: {}", err_msg);
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 4. 拖拽悬停实时计算回调
    // -------------------------------------------------------------------------
    // 鼠标在列表中拖拽悬停移动时触发，用于实时计算目标节点是否合法并计算高亮吸附下划线/边框位置。
    let window_weak = window.as_weak();
    let master_tree_hover = Rc::clone(&ctx.master_tree);
    let master_cards_hover = Rc::clone(&ctx.master_cards);
    let expanded_hover = Rc::clone(&ctx.expanded_groups);
    let search_hover = Rc::clone(&ctx.search_query);
    window.on_request_drag_hover(move |src_id, target_idx, _offset_in_row| {
        if let Some(w) = window_weak.upgrade() {
            let src_str = src_id.to_string();
            let view_mode = w.get_hosts_view_mode().to_string();

            // 1. 卡片模式悬停判定
            if view_mode == "card" {
                let cards = master_cards_hover.borrow();
                let idx = target_idx as usize;
                if idx < cards.len() {
                    let tgt_id = cards[idx].id.to_string();
                    if tgt_id != src_str {
                        w.set_drop_target_id(tgt_id.into());
                        w.set_drop_position("after".into());
                        w.set_drop_target_valid(true);
                        w.set_drop_target_index(target_idx);
                    } else {
                        w.set_drop_target_id("".into());
                        w.set_drop_position("none".into());
                        w.set_drop_target_valid(false);
                        w.set_drop_target_index(-1);
                    }
                } else {
                    w.set_drop_target_id("".into());
                    w.set_drop_position("none".into());
                    w.set_drop_target_valid(false);
                    w.set_drop_target_index(-1);
                }
                return;
            }

            // 2. 树形模式悬停判定
            let tree = master_tree_hover.borrow();
            let q = search_hover.borrow().clone();
            let visible_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_hover.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };

            // 拖拽至顶部“移至根目录 (未分组)”区域判定
            if target_idx < 0 {
                w.set_drop_target_id("root".into());
                w.set_drop_position("root".into());
                w.set_drop_target_valid(true);
                w.set_drop_target_index(-1);
                return;
            }

            let idx = target_idx as usize;
            if idx >= visible_nodes.len() {
                w.set_drop_target_id("".into());
                w.set_drop_position("none".into());
                w.set_drop_target_valid(false);
                w.set_drop_target_index(-1);
                return;
            }

            let target_node = &visible_nodes[idx];
            let tgt_id = target_node.id.to_string();

            // 防呆规则 1: 禁止拖拽放置到自身节点
            if tgt_id == src_str {
                w.set_drop_target_id("".into());
                w.set_drop_position("none".into());
                w.set_drop_target_valid(false);
                w.set_drop_target_index(-1);
                return;
            }

            // 防呆规则 2: 防止循环嵌套（禁止将父级分组拖入自己的后代子分组中）
            let mut curr = tgt_id.clone();
            let mut is_descendant = false;
            while !curr.is_empty() {
                if curr == src_str {
                    is_descendant = true;
                    break;
                }
                if let Some(p) = tree.iter().find(|n| n.id == curr) {
                    curr = p.parent_id.clone();
                } else {
                    break;
                }
            }
            if is_descendant {
                w.set_drop_target_id("".into());
                w.set_drop_position("none".into());
                w.set_drop_target_valid(false);
                w.set_drop_target_index(-1);
                return;
            }

            // 规则 3: 确定悬停有效落点 (文件夹高亮内部放置，主机高亮下插线)
            w.set_drop_target_id(tgt_id.into());
            w.set_drop_target_index(target_idx);
            w.set_drop_target_valid(true);
            if target_node.is_group {
                w.set_drop_position("inside".into());
            } else {
                w.set_drop_position("after".into());
            }

        }
    });

    // -------------------------------------------------------------------------
    // 5. 新建分组模态对话框提交回调
    // -------------------------------------------------------------------------
    // 接收弹窗输入的分组名称与指定父级 ID，在树中创建分组并同步持久化到 AppStorage。
    let window_weak = window.as_weak();
    let master_tree_create = Rc::clone(&ctx.master_tree);
    let expanded_create = Rc::clone(&ctx.expanded_groups);
    let selector_expanded_create = Rc::clone(&ctx.selector_expanded_groups);
    let search_query_create = Rc::clone(&ctx.search_query);
    let next_group_id = Rc::clone(&ctx.next_session_num);
    let core_state_create = Rc::clone(&ctx.core_state);
    window.on_create_group(move |parent_id, name| {
        if let Some(w) = window_weak.upgrade() {
            let g_name = name.trim().to_string();
            let p_id = parent_id.trim().to_string();
            if g_name.is_empty() {
                return;
            }

            let mut counter = next_group_id.borrow_mut();
            *counter += 1;
            let new_id = format!("grp-custom-{}", *counter);

            let mut tree = master_tree_create.borrow_mut();

            // 如果指定了父分组，计算层级与父 ID
            let (target_parent_id, level) = if !p_id.is_empty() && p_id != "root" {
                if let Some(parent_node) = tree.iter().find(|n| n.id == p_id) {
                    (p_id.clone(), parent_node.level + 1)
                } else {
                    ("".to_string(), 0)
                }
            } else {
                ("".to_string(), 0)
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

            // 同步新增分组至底层存储层
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
        }
    });

    // -------------------------------------------------------------------------
    // 6. 主机实时搜索过滤回调
    // -------------------------------------------------------------------------
    // 当在左侧抽屉搜索框中键入字符时，双向联动过滤树形视图与卡片列表，同时保持各自排布顺序。
    let window_weak = window.as_weak();
    let master_tree_filter = Rc::clone(&ctx.master_tree);
    let master_cards_filter = Rc::clone(&ctx.master_cards);
    let expanded_clone = Rc::clone(&ctx.expanded_groups);
    let search_query_filter = Rc::clone(&ctx.search_query);
    let core_state_filter = Rc::clone(&ctx.core_state);
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
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(filtered_cards.clone()))));

            core_state_filter.events().dispatch(&HostSearchFilteredEvent {
                query: q.clone(),
                match_count: filtered_cards.len(),
            });

            if !q.is_empty() {
                tracing::debug!(target: "smagical_ui::search", "过滤主机资产: '{}'", q);
            }
        }

    });

    // -------------------------------------------------------------------------
    // 7. 打开主机终端会话回调
    // -------------------------------------------------------------------------
    // 8. 双击主机 / 本地 Shell 发起终端连接回调
    // -------------------------------------------------------------------------
    // 双击树形或卡片列表中的某个主机（或选择本地 Shell）时触发，分配会话 ID 并激活新 Tab。
    let window_weak = window.as_weak();
    let master_tree_open = Rc::clone(&ctx.master_tree);
    let pane_groups_open = Rc::clone(&ctx.pane_groups);
    let active_pane_id_open = Rc::clone(&ctx.active_pane_id);
    let global_split_tree_open = Rc::clone(&ctx.global_split_tree);
    let active_terminals_open = Rc::clone(&ctx.active_terminals);
    let next_session_num_open = Rc::clone(&ctx.next_session_num);
    let next_pane_num_open = Rc::clone(&ctx.next_pane_num);
    let cached_shells_open = std::sync::Arc::clone(&ctx.cached_shells);

    let ctx_open = ctx.clone();
    window.on_open_host(move |host_id| {

        if let Some(w) = window_weak.upgrade() {
            let h_id = host_id.to_string();

            let (sess_id, info) = if h_id.starts_with("local-") {
                let mut num = next_session_num_open.borrow_mut();
                let sess_id = format!("sess-{}", *num);
                *num += 1;

                let all_shells = cached_shells_open.read().unwrap();
                let (base_name, addr) = if let Some(sh) = all_shells.iter().find(|s| s.id == h_id.as_str()) {

                    (sh.title.to_string(), format!("Local ({})", sh.subtitle))
                } else {
                    ("Local Terminal".to_string(), "127.0.0.1".to_string())
                };

                let mut total_sess_count = 0;
                for g in pane_groups_open.borrow().iter() {
                    total_sess_count += g.tabs.len();
                }
                let session_name = format!("{} #{}", base_name, total_sess_count + 1);

                if let Ok(instance) = TerminalInstance::spawn_local(sess_id.clone(), &h_id, session_name.clone(), 120, 32) {
                    active_terminals_open.borrow_mut().insert(sess_id.clone(), instance);
                }

                let info = TerminalSessionInfo {
                    session_id: sess_id.clone(),
                    host_id: h_id.clone(),
                    host_name: base_name.clone(),
                    host_address: addr,
                    host_status: "online".to_string(),
                    ping_ms: 0,
                    display_title: session_name,
                };
                (sess_id, info)
            } else {
                let tree = master_tree_open.borrow();
                let Some(host_node) = tree.iter().find(|n| n.id == h_id && !n.is_group) else {
                    return;
                };

                let mut num = next_session_num_open.borrow_mut();
                let sess_id = format!("sess-{}", *num);
                *num += 1;

                let mut total_sess_count = 0;
                for g in pane_groups_open.borrow().iter() {
                    total_sess_count += g.tabs.len();
                }
                let session_name = format!("{} #{}", host_node.name, total_sess_count + 1);

                let instance_res = TerminalInstance::spawn_ssh(
                    sess_id.clone(),
                    session_name.clone(),
                    &host_node.address,
                    host_node.port as u16,
                    None,
                    120,
                    32,
                )
                .or_else(|_| {
                    TerminalInstance::spawn_local(
                        sess_id.clone(),
                        "local-powershell",
                        session_name.clone(),
                        120,
                        32,
                    )
                });

                if let Ok(instance) = instance_res {
                    active_terminals_open.borrow_mut().insert(sess_id.clone(), instance);
                }

                let info = TerminalSessionInfo {
                    session_id: sess_id.clone(),
                    host_id: host_node.id.clone(),
                    host_name: host_node.name.clone(),
                    host_address: host_node.address.clone(),
                    host_status: host_node.status.clone(),
                    ping_ms: host_node.ping_ms,
                    display_title: session_name,
                };
                (sess_id, info)
            };

            // 广播终端会话已开启事件
            ctx_open.core_state.events().dispatch(&TerminalSessionEvent {
                session_id: sess_id.clone(),
                host_id: info.host_id.clone(),
                action: "opened".into(),
            });
            crate::handlers::history_handlers::sync_ui_history(&w, &ctx_open);





            let mut groups = pane_groups_open.borrow_mut();
            let mut active_pid = active_pane_id_open.borrow_mut();
            let is_split = global_split_tree_open.borrow().is_some();

            if groups.is_empty() {
                let mut p_num = next_pane_num_open.borrow_mut();
                let pid = format!("pane-{}", *p_num);
                *p_num += 1;
                groups.push(crate::session::PaneGroup::new_single(pid.clone(), info));
                *active_pid = pid;
            } else {
                let target_idx = groups.iter().position(|g| g.pane_id == *active_pid).unwrap_or(0);
                let target_group = &mut groups[target_idx];
                let insert_idx = if let Some(pos) = target_group.tabs.iter().position(|s| s.session_id == target_group.active_tab_id) {
                    pos + 1
                } else {
                    target_group.tabs.len()
                };
                target_group.tabs.insert(insert_idx, info);
                target_group.active_tab_id = sess_id;
                *active_pid = target_group.pane_id.clone();
            }

            sync_active_session_ui(&w, &groups, &active_pid, is_split);
            crate::session::sync_active_session_to_core(&groups, &active_pid, &ctx_open.core_state);
            tracing::info!(target: "smagical_ui::session", "成功打开终端会话 (Pane ID: {})", *active_pid);
        }
    });
}






