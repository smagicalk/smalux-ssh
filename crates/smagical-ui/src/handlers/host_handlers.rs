//! 主机与分组资产管理、树形/列表视图拖拽移动、快速打开终端等交互回调绑定。
//!
//! 包含多级分组嵌套维护、无环拓扑防呆校验、平滑调序与动态视口宽度计算。

use std::net::ToSocketAddrs;
use std::rc::Rc;
use slint::ComponentHandle;
use smagical_core::event::{
    HostAssetChangedEvent, HostGroupToggledEvent, HostSearchFilteredEvent, HostTreeReorderedEvent,
    TerminalSessionEvent,
};
use smagical_core::GroupRecord;

use crate::generated::{
    AppWindow, CredentialOptionData, FilesBridge, GroupOptionData, HostItemData, HostTreeNode, HostsBridge,
    JumpHostOptionData, JumpHopItemData, PresetJumpChainData, ProxyOptionData, WindowBridge,
};
use crate::handlers::AppContext;
use crate::session::{sync_active_session_ui, TerminalSessionInfo};
use crate::terminal::TerminalInstance;
use crate::tree_model::{
    build_group_options, build_raw_tree_from_storage, build_search_tree_nodes,
    build_visible_tree_nodes, calculate_max_tree_width, move_and_reorder_raw_node, RawTreeNode,
};

fn sync_hosts_bridge_tree(w: &AppWindow, nodes: &[HostTreeNode]) {
    let hb = w.global::<HostsBridge>();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(nodes.to_vec())));
    let width = calculate_max_tree_width(nodes);
    hb.set_tree_nodes(model);
    hb.set_tree_content_width(width);
}

fn sync_hosts_bridge_cards(w: &AppWindow, cards: &[HostItemData]) {
    let hb = w.global::<HostsBridge>();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(cards.to_vec())));
    hb.set_hosts(model);
}

fn sync_hosts_bridge_options(w: &AppWindow, options: &[GroupOptionData]) {
    let hb = w.global::<HostsBridge>();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(options.to_vec())));
    hb.set_group_options(model);
}

fn sync_hosts_bridge_credentials(w: &AppWindow, options: &[CredentialOptionData]) {
    let hb = w.global::<HostsBridge>();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(options.to_vec())));
    hb.set_credential_options(model);
}

fn sync_hosts_bridge_jump_hosts(w: &AppWindow, options: &[JumpHostOptionData]) {
    let hb = w.global::<HostsBridge>();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(options.to_vec())));
    hb.set_jump_host_options(model);
}

fn sync_hosts_bridge_preset_jump_chains(w: &AppWindow, options: &[PresetJumpChainData]) {
    let hb = w.global::<HostsBridge>();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(options.to_vec())));
    hb.set_preset_jump_chains(model);
}

fn sync_hosts_bridge_proxies(w: &AppWindow, options: &[ProxyOptionData]) {
    let hb = w.global::<HostsBridge>();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(options.to_vec())));
    hb.set_proxy_options(model);
}

fn sync_hosts_bridge_create_host_jump_chain(w: &AppWindow, chain: &[JumpHopItemData]) {
    let hb = w.global::<HostsBridge>();
    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(chain.to_vec())));
    hb.set_create_host_jump_chain(model);
}


/// 注册主机资产管理相关交互回调。
///
/// 绑定分组折叠/展开、跨分组拖拽迁移、弹窗选择器级联、实时搜索过滤及双击打开终端等事件。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用
/// - `ctx`: 全局应用共享上下文对象引用
pub(crate) fn register_host_handlers(window: &AppWindow, ctx: &AppContext) {
    let hb = window.global::<HostsBridge>();
    // -------------------------------------------------------------------------
    // 1. 新建/编辑主机弹窗中“上级分组选择器”折叠 / 展开回调
    // -------------------------------------------------------------------------
    // 支持在新建主机或新建分组弹窗的下拉树形选择框内收缩或展开某个父级节点。
    let window_weak = window.as_weak();
    let master_tree_toggle_opt = Rc::clone(&ctx.master_tree);
    let selector_expanded_clone = Rc::clone(&ctx.selector_expanded_groups);
    hb.on_toggle_selector_group(move |id| {
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
            sync_hosts_bridge_options(&w, &next_options);
        }
    });

    // -------------------------------------------------------------------------
    // 2. 侧边栏树形结构分组折叠 / 展开回调
    // -------------------------------------------------------------------------
    // 点击左侧主机树中的某个文件夹节点时触发，切换展开状态并同步持久化至 AppStorage。
    let window_weak = window.as_weak();
    let master_tree_toggle = Rc::clone(&ctx.master_tree);
    let expanded_toggle = Rc::clone(&ctx.expanded_groups);
    let search_query_toggle = Rc::clone(&ctx.search_query);
    let core_state_toggle = Rc::clone(&ctx.core_state);
    hb.on_toggle_group(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let mut set = expanded_toggle.borrow_mut();
            let id_str = id.to_string();
            let is_expanding = if set.contains(&id_str) {
                set.remove(&id_str);
                false
            } else {
                set.insert(id_str.clone());
                true
            };

            // 同步持久化分组折叠/展开状态至存储层
            let _ = core_state_toggle.storage().groups().set_expanded(&id_str, is_expanding);

            // 显式派发分组折叠/展开事件
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
            sync_hosts_bridge_tree(&w, &next_nodes);

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
    hb.on_move_node(move |src_id, target_id, drop_position| {
        if let Some(w) = window_weak.upgrade() {
            let src_str = src_id.to_string();
            let target_str = target_id.to_string();
            let pos_str = drop_position.to_string();
            let view_mode = w.global::<HostsBridge>().get_hosts_view_mode().to_string();


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
                    sync_hosts_bridge_cards(&w, &display_cards);

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
                    sync_hosts_bridge_tree(&w, &next_nodes);

                    let next_options = build_group_options(&tree, &selector_expanded_move.borrow());
                    sync_hosts_bridge_options(&w, &next_options);

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
                    sync_hosts_bridge_cards(&w, &display_cards);

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
    hb.on_request_drag_hover(move |src_id, target_idx, _offset_in_row| {
        if let Some(w) = window_weak.upgrade() {
            let hb = w.global::<HostsBridge>();
            let src_str = src_id.to_string();
            let view_mode = hb.get_hosts_view_mode().to_string();

            // 1. 卡片模式悬停判定
            if view_mode == "card" {
                let cards = master_cards_hover.borrow();
                let idx = target_idx as usize;
                if idx < cards.len() {
                    let tgt_id = cards[idx].id.to_string();
                    if tgt_id != src_str {
                        hb.set_drop_target_id(tgt_id.into());
                        hb.set_drop_position("after".into());
                        hb.set_drop_target_valid(true);
                        hb.set_drop_target_index(target_idx);
                    } else {
                        hb.set_drop_target_id("".into());
                        hb.set_drop_position("none".into());
                        hb.set_drop_target_valid(false);
                        hb.set_drop_target_index(-1);
                    }
                } else {
                    hb.set_drop_target_id("".into());
                    hb.set_drop_position("none".into());
                    hb.set_drop_target_valid(false);
                    hb.set_drop_target_index(-1);
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
                hb.set_drop_target_id("root".into());
                hb.set_drop_position("root".into());
                hb.set_drop_target_valid(true);
                hb.set_drop_target_index(-1);
                return;
            }

            let idx = target_idx as usize;
            if idx >= visible_nodes.len() {
                hb.set_drop_target_id("".into());
                hb.set_drop_position("none".into());
                hb.set_drop_target_valid(false);
                hb.set_drop_target_index(-1);
                return;
            }

            let target_node = &visible_nodes[idx];
            let tgt_id = target_node.id.to_string();

            // 防呆规则 1: 禁止拖拽放置到自身节点
            if tgt_id == src_str {
                hb.set_drop_target_id("".into());
                hb.set_drop_position("none".into());
                hb.set_drop_target_valid(false);
                hb.set_drop_target_index(-1);
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
                hb.set_drop_target_id("".into());
                hb.set_drop_position("none".into());
                hb.set_drop_target_valid(false);
                hb.set_drop_target_index(-1);
                return;
            }

            // 规则 3: 确定悬停有效落点 (文件夹高亮内部放置，主机高亮下插线)
            hb.set_drop_target_id(tgt_id.into());
            hb.set_drop_target_index(target_idx);
            hb.set_drop_target_valid(true);
            if target_node.is_group {
                hb.set_drop_position("inside".into());
            } else {
                hb.set_drop_position("after".into());
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
    hb.on_create_group(move |parent_id, name| {
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
            sync_hosts_bridge_options(&w, &next_options);

            // 刷新主界面树形结构
            let q = search_query_create.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_create.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            sync_hosts_bridge_tree(&w, &next_nodes);

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
    hb.on_search_changed(move |query| {
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
            sync_hosts_bridge_tree(&w, &next_nodes);

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
            sync_hosts_bridge_cards(&w, &filtered_cards);

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
    hb.on_open_host(move |host_id| {

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

                // 查询主机是否有关联凭据，获取用户名（优先使用凭据，无凭据或未设时回退使用主机直录账号）
                let username_opt = if let Ok(Some(host_rec)) = ctx_open.core_state.storage().hosts().get_by_id(&h_id) {
                    if let Some(ref cred_id) = host_rec.credential_id {
                        if let Ok(Some(cred)) = ctx_open.core_state.storage().credentials().get_by_id(cred_id) {
                            cred.username.or(host_rec.username)
                        } else {
                            host_rec.username
                        }
                    } else {
                        host_rec.username
                    }
                } else {
                    None
                };

                let instance_res = TerminalInstance::spawn_ssh(
                    sess_id.clone(),
                    session_name.clone(),
                    &host_node.address,
                    host_node.port as u16,
                    username_opt.as_deref(),
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

    // -------------------------------------------------------------------------
    // 9. 领域总线 HostsBridge 基础状态响应
    // -------------------------------------------------------------------------
    {
        let window_weak = window.as_weak();
        hb.on_select_host(move |id| {
            if let Some(w) = window_weak.upgrade() {
                w.global::<HostsBridge>().set_selected_host_id(id);
            }
        });
    }
    {
        let window_weak = window.as_weak();
        hb.on_switch_view_mode(move |mode| {
            if let Some(w) = window_weak.upgrade() {
                w.global::<HostsBridge>().set_hosts_view_mode(mode);
            }
        });
    }
    {
        let window_weak = window.as_weak();
        hb.on_close_group_modal(move || {
            if let Some(w) = window_weak.upgrade() {
                w.global::<HostsBridge>().set_is_create_group_open(false);
            }
        });
    }
    {
        let window_weak = window.as_weak();
        hb.on_create_new_group(move || {
            if let Some(w) = window_weak.upgrade() {
                w.global::<HostsBridge>().set_is_create_group_open(true);
            }
        });
    }

    // -------------------------------------------------------------------------
    // 10. 呼出新建主机模态弹窗 (加载凭据库、跳板机与分组选项)
    // -------------------------------------------------------------------------
    let window_weak_open_host_modal = window.as_weak();
    let core_state_open_host_modal = Rc::clone(&ctx.core_state);
    let master_tree_open_host_modal = Rc::clone(&ctx.master_tree);
    let selector_expanded_open_host_modal = Rc::clone(&ctx.selector_expanded_groups);
    let create_host_jump_chain = Rc::new(std::cell::RefCell::new(Vec::<JumpHopItemData>::new()));
    let create_host_jump_chain_open = Rc::clone(&create_host_jump_chain);

    let open_host_modal_fn = move |default_parent: slint::SharedString| {
        if let Some(w) = window_weak_open_host_modal.upgrade() {
            let hb = w.global::<HostsBridge>();
            let p_id = if default_parent.is_empty() { "root".to_string() } else { default_parent.to_string() };
            hb.set_create_host_default_parent(p_id.clone().into());
            hb.set_test_connection_status("idle".into());
            hb.set_test_connection_message("".into());

            // 1. 初始化为新建模式，重置所有表单字段
            hb.set_is_edit_mode(false);
            hb.set_editing_host_id("".into());
            hb.set_form_host_name("".into());
            hb.set_form_host_address("".into());
            hb.set_form_host_port_str("22".into());
            hb.set_form_parent_id(p_id.clone().into());
            let p_name = if p_id == "root" {
                "根目录 (顶级主机)".to_string()
            } else {
                core_state_open_host_modal.storage().groups().get_by_id(&p_id).ok().flatten()
                    .map(|g| g.name)
                    .unwrap_or_else(|| "根目录 (顶级主机)".to_string())
            };
            hb.set_form_parent_name(p_name.into());
            hb.set_form_auth_type("credential".into());
            hb.set_form_cred_id("".into());
            hb.set_form_cred_name("".into());
            hb.set_form_cred_user("".into());
            hb.set_form_cred_type("".into());
            hb.set_form_cred_alg("".into());
            hb.set_form_auth_username("root".into());
            hb.set_form_auth_password("".into());
            hb.set_form_auth_key_username("root".into());
            hb.set_form_auth_key_data("".into());
            hb.set_form_auth_key_passphrase("".into());
            hb.set_form_proxy_mode("direct".into());
            hb.set_form_custom_proxy_proto("socks5".into());
            hb.set_form_custom_proxy_host("".into());
            hb.set_form_custom_proxy_port_str("".into());
            hb.set_form_custom_proxy_username("".into());
            hb.set_form_custom_proxy_password("".into());
            hb.set_form_keepalive_str("30".into());
            hb.set_form_initial_dir("".into());
            hb.set_form_startup_cmd("".into());
            hb.set_form_term_type("xterm-256color".into());
            hb.set_form_notes("".into());
            hb.set_form_preset_jump_id("".into());
            hb.set_form_preset_jump_name("".into());
            hb.set_form_preset_jump_summary("".into());
            hb.set_form_preset_jump_hop_count(0);

            // 2. 同步最新分组树形选项
            let tree = master_tree_open_host_modal.borrow();
            let expanded_set = selector_expanded_open_host_modal.borrow();
            let group_options = build_group_options(&tree, &expanded_set);
            sync_hosts_bridge_options(&w, &group_options);

            // 3. 读取并同步凭据库选项
            if let Ok(creds) = core_state_open_host_modal.storage().credentials().list_all() {
                let cred_options: Vec<CredentialOptionData> = creds.into_iter().map(|c| {
                    CredentialOptionData {
                        id: c.id.into(),
                        name: c.name.into(),
                        username: c.username.unwrap_or_else(|| "root".to_string()).into(),
                        cred_type: c.cred_type.to_string().into(),
                        algorithm: c.algorithm.into(),
                    }
                }).collect();
                sync_hosts_bridge_credentials(&w, &cred_options);
            }

            // 4. 读取并同步可用跳板主机列表
            if let Ok(hosts) = core_state_open_host_modal.storage().hosts().list_all() {
                let jump_options: Vec<JumpHostOptionData> = hosts.into_iter().map(|h| {
                    JumpHostOptionData {
                        id: h.id.into(),
                        name: h.name.into(),
                        address: h.address.into(),
                        port: h.port as i32,
                    }
                }).collect();
                sync_hosts_bridge_jump_hosts(&w, &jump_options);
            }

            // 5. 从网络与隧道中心读取组好的跳板串列表 (TunnelType::JumpHost)
            if let Ok(tunnels) = core_state_open_host_modal.storage().tunnels().list_all() {
                let preset_chains: Vec<PresetJumpChainData> = tunnels.iter()
                    .filter(|t| t.tunnel_type == smagical_core::TunnelType::JumpHost)
                    .map(|t| {
                        let hops_str = t.jump_chain.iter()
                            .map(|h| format!("{},{},{},{}", h.host_id, h.host_name, h.host_address, h.host_port))
                            .collect::<Vec<_>>()
                            .join(";");
                        PresetJumpChainData {
                            id: t.id.clone().into(),
                            name: t.name.clone().into(),
                            summary: t.route_summary().into(),
                            hop_count: t.jump_chain.len() as i32,
                            hops_serialized: hops_str.into(),
                        }
                    })
                    .collect();
                sync_hosts_bridge_preset_jump_chains(&w, &preset_chains);

                // 6. 从网络与隧道中心读取已有代理配置 (TunnelType::ProxyServer 或 Dynamic)
                let proxies: Vec<ProxyOptionData> = tunnels.iter()
                    .filter(|t| t.tunnel_type == smagical_core::TunnelType::ProxyServer || t.tunnel_type == smagical_core::TunnelType::Dynamic)
                    .map(|t| {
                        let proto = if t.tunnel_type == smagical_core::TunnelType::Dynamic {
                            "SOCKS5".to_string()
                        } else if !t.proxy_proto.is_empty() {
                            t.proxy_proto.to_uppercase()
                        } else {
                            "SOCKS5".to_string()
                        };
                        let (host, port) = if t.tunnel_type == smagical_core::TunnelType::Dynamic {
                            let h = if !t.local_bind.is_empty() { t.local_bind.clone() } else { "127.0.0.1".to_string() };
                            (h, t.local_port as i32)
                        } else {
                            let h = if !t.remote_host.is_empty() { t.remote_host.clone() } else if !t.local_bind.is_empty() { t.local_bind.clone() } else { "127.0.0.1".to_string() };
                            let p = if t.remote_port > 0 { t.remote_port as i32 } else { t.local_port as i32 };
                            (h, p)
                        };
                        ProxyOptionData {
                            id: t.id.clone().into(),
                            name: t.name.clone().into(),
                            proto: proto.into(),
                            host: host.into(),
                            port,
                            username: t.proxy_username.clone().into(),
                        }
                    })
                    .collect();
                sync_hosts_bridge_proxies(&w, &proxies);
            }

            // 7. 重置当前跳板机链路
            create_host_jump_chain_open.borrow_mut().clear();
            sync_hosts_bridge_create_host_jump_chain(&w, &[]);

            hb.set_is_create_host_modal_open(true);
        }
    };

    let open_host_modal_clone = open_host_modal_fn.clone();
    hb.on_open_create_host_modal(move |def_parent| {
        open_host_modal_clone(def_parent);
    });

    let open_host_modal_create = open_host_modal_fn;
    hb.on_create_new_host(move || {
        open_host_modal_create("root".into());
    });

    // -------------------------------------------------------------------------
    // 10.0 右键菜单 - 呼出编辑主机模态弹窗 (回显已保存主机字段，共用组件)
    // -------------------------------------------------------------------------
    let window_weak_edit_modal = window.as_weak();
    let core_state_edit_modal = Rc::clone(&ctx.core_state);
    let master_tree_edit_modal = Rc::clone(&ctx.master_tree);
    let selector_expanded_edit_modal = Rc::clone(&ctx.selector_expanded_groups);
    let create_host_jump_chain_edit = Rc::clone(&create_host_jump_chain);
    let notifications_edit_modal = ctx.notifications.clone();

    hb.on_open_edit_host_modal(move |host_id| {
        if let Some(w) = window_weak_edit_modal.upgrade() {
            let h_id = host_id.to_string();
            let host_opt = core_state_edit_modal.storage().hosts().get_by_id(&h_id).ok().flatten();
            let host = match host_opt {
                Some(h) => h,
                None => {
                    notifications_edit_modal.error("打开失败", format!("未找到主机 [{}] 的记录", h_id));
                    return;
                }
            };

            let hb = w.global::<HostsBridge>();
            hb.set_test_connection_status("idle".into());
            hb.set_test_connection_message("".into());

            // 1. 设置为编辑模式并绑定原主机 ID
            hb.set_is_edit_mode(true);
            hb.set_editing_host_id(host.id.clone().into());

            // 2. 解析回显所属分组
            let (parent_id, parent_name) = if let Some(ref pid) = host.parent_group_id {
                let name = core_state_edit_modal.storage().groups().get_by_id(pid).ok().flatten()
                    .map(|g| g.name)
                    .unwrap_or_else(|| "根目录 (顶级主机)".to_string());
                (pid.clone(), name)
            } else {
                ("root".to_string(), "根目录 (顶级主机)".to_string())
            };
            hb.set_create_host_default_parent(parent_id.clone().into());
            hb.set_form_parent_id(parent_id.into());
            hb.set_form_parent_name(parent_name.into());

            // 3. 常规信息回显
            hb.set_form_host_name(host.name.clone().into());
            hb.set_form_host_address(host.address.clone().into());
            hb.set_form_host_port_str(host.port.to_string().into());

            // 4. 凭据与认证模式回显
            let (cred_id, cred_name, cred_user, cred_type, cred_alg) = if let Some(ref cid) = host.credential_id {
                if let Ok(Some(c)) = core_state_edit_modal.storage().credentials().get_by_id(cid) {
                    (
                        c.id,
                        c.name,
                        c.username.unwrap_or_else(|| "root".to_string()),
                        c.cred_type.to_string(),
                        c.algorithm,
                    )
                } else {
                    (cid.clone(), cid.clone(), "root".to_string(), "password".to_string(), "Password".to_string())
                }
            } else {
                (String::new(), String::new(), String::new(), String::new(), String::new())
            };
            hb.set_form_cred_id(cred_id.into());
            hb.set_form_cred_name(cred_name.into());
            hb.set_form_cred_user(cred_user.into());
            hb.set_form_cred_type(cred_type.into());
            hb.set_form_cred_alg(cred_alg.into());

            let at = if host.auth_type.is_empty() {
                if host.credential_id.is_some() {
                    "credential".to_string()
                } else if host.key_data.is_some() {
                    "key".to_string()
                } else {
                    "password".to_string()
                }
            } else {
                host.auth_type.clone()
            };
            hb.set_form_auth_type(at.into());
            hb.set_form_auth_username(host.username.clone().unwrap_or_else(|| "root".to_string()).into());
            hb.set_form_auth_password(host.password.clone().unwrap_or_default().into());
            hb.set_form_auth_key_username(host.username.clone().unwrap_or_else(|| "root".to_string()).into());
            hb.set_form_auth_key_data(host.key_data.clone().unwrap_or_default().into());
            hb.set_form_auth_key_passphrase(host.key_passphrase.clone().unwrap_or_default().into());

            // 5. 代理配置回显
            let (proxy_mode, custom_proto, custom_host, custom_port_str, custom_user, custom_pass) =
                if let Some(ref ptype) = host.proxy_type {
                    (
                        "custom".to_string(),
                        ptype.clone(),
                        host.proxy_host.clone().unwrap_or_default(),
                        host.proxy_port.map(|p| p.to_string()).unwrap_or_default(),
                        host.proxy_username.clone().unwrap_or_default(),
                        host.proxy_password.clone().unwrap_or_default(),
                    )
                } else {
                    ("direct".to_string(), "socks5".to_string(), String::new(), String::new(), String::new(), String::new())
                };
            hb.set_form_proxy_mode(proxy_mode.into());
            hb.set_form_custom_proxy_proto(custom_proto.into());
            hb.set_form_custom_proxy_host(custom_host.into());
            hb.set_form_custom_proxy_port_str(custom_port_str.into());
            hb.set_form_custom_proxy_username(custom_user.into());
            hb.set_form_custom_proxy_password(custom_pass.into());

            // 6. 高级参数回显
            hb.set_form_keepalive_str(host.keepalive_interval.to_string().into());
            hb.set_form_initial_dir(host.initial_dir.clone().unwrap_or_default().into());
            hb.set_form_startup_cmd(host.startup_cmd.clone().unwrap_or_default().into());
            hb.set_form_term_type(host.term_type.clone().unwrap_or_else(|| "xterm-256color".to_string()).into());
            hb.set_form_notes(host.notes.clone().into());

            // 7. 跳板链路预设回显
            create_host_jump_chain_edit.borrow_mut().clear();
            if let Some(ref jsummary) = host.jump_chain_summary {
                if jsummary.starts_with("preset:") {
                    let pid = jsummary.trim_start_matches("preset:");
                    hb.set_form_preset_jump_id(pid.into());
                    if let Ok(Some(tun)) = core_state_edit_modal.storage().tunnels().get_by_id(pid) {
                        let summary = tun.route_summary();
                        let hop_count = tun.jump_chain.len() as i32;
                        hb.set_form_preset_jump_name(tun.name.into());
                        hb.set_form_preset_jump_summary(summary.into());
                        hb.set_form_preset_jump_hop_count(hop_count);
                    }
                } else {
                    hb.set_form_preset_jump_id("".into());
                    hb.set_form_preset_jump_name("".into());
                    hb.set_form_preset_jump_summary("".into());
                    hb.set_form_preset_jump_hop_count(0);
                }
            } else {
                hb.set_form_preset_jump_id("".into());
                hb.set_form_preset_jump_name("".into());
                hb.set_form_preset_jump_summary("".into());
                hb.set_form_preset_jump_hop_count(0);
            }
            sync_hosts_bridge_create_host_jump_chain(&w, &[]);

            // 8. 同步下拉选项数据
            {
                let tree = master_tree_edit_modal.borrow();
                let expanded_set = selector_expanded_edit_modal.borrow();
                let group_options = build_group_options(&tree, &expanded_set);
                sync_hosts_bridge_options(&w, &group_options);
            }
            if let Ok(creds) = core_state_edit_modal.storage().credentials().list_all() {
                let cred_options: Vec<CredentialOptionData> = creds.into_iter().map(|c| {
                    CredentialOptionData {
                        id: c.id.into(),
                        name: c.name.into(),
                        username: c.username.unwrap_or_else(|| "root".to_string()).into(),
                        cred_type: c.cred_type.to_string().into(),
                        algorithm: c.algorithm.into(),
                    }
                }).collect();
                sync_hosts_bridge_credentials(&w, &cred_options);
            }
            if let Ok(hosts) = core_state_edit_modal.storage().hosts().list_all() {
                let jump_options: Vec<JumpHostOptionData> = hosts.into_iter().map(|h| {
                    JumpHostOptionData {
                        id: h.id.into(),
                        name: h.name.into(),
                        address: h.address.into(),
                        port: h.port as i32,
                    }
                }).collect();
                sync_hosts_bridge_jump_hosts(&w, &jump_options);
            }
            if let Ok(tunnels) = core_state_edit_modal.storage().tunnels().list_all() {
                let preset_chains: Vec<PresetJumpChainData> = tunnels.iter()
                    .filter(|t| t.tunnel_type == smagical_core::TunnelType::JumpHost)
                    .map(|t| {
                        let hops_str = t.jump_chain.iter()
                            .map(|h| format!("{},{},{},{}", h.host_id, h.host_name, h.host_address, h.host_port))
                            .collect::<Vec<_>>()
                            .join(";");
                        PresetJumpChainData {
                            id: t.id.clone().into(),
                            name: t.name.clone().into(),
                            summary: t.route_summary().into(),
                            hop_count: t.jump_chain.len() as i32,
                            hops_serialized: hops_str.into(),
                        }
                    })
                    .collect();
                sync_hosts_bridge_preset_jump_chains(&w, &preset_chains);

                let proxies: Vec<ProxyOptionData> = tunnels.iter()
                    .filter(|t| t.tunnel_type == smagical_core::TunnelType::ProxyServer || t.tunnel_type == smagical_core::TunnelType::Dynamic)
                    .map(|t| {
                        let proto = if t.tunnel_type == smagical_core::TunnelType::Dynamic {
                            "SOCKS5".to_string()
                        } else if !t.proxy_proto.is_empty() {
                            t.proxy_proto.to_uppercase()
                        } else {
                            "SOCKS5".to_string()
                        };
                        let (host, port) = if t.tunnel_type == smagical_core::TunnelType::Dynamic {
                            let h = if !t.local_bind.is_empty() { t.local_bind.clone() } else { "127.0.0.1".to_string() };
                            (h, t.local_port as i32)
                        } else {
                            let h = if !t.remote_host.is_empty() { t.remote_host.clone() } else if !t.local_bind.is_empty() { t.local_bind.clone() } else { "127.0.0.1".to_string() };
                            let p = if t.remote_port > 0 { t.remote_port as i32 } else { t.local_port as i32 };
                            (h, p)
                        };
                        ProxyOptionData {
                            id: t.id.clone().into(),
                            name: t.name.clone().into(),
                            proto: proto.into(),
                            host: host.into(),
                            port,
                            username: t.proxy_username.clone().into(),
                        }
                    })
                    .collect();
                sync_hosts_bridge_proxies(&w, &proxies);
            }

            hb.set_is_create_host_modal_open(true);
        }
    });


    // -------------------------------------------------------------------------
    // 10.1 动态跳板链路节点操作回调 (添加/导入预设串/上移/下移/删除/清空)
    // -------------------------------------------------------------------------
    {
        let window_weak = window.as_weak();
        let chain_add = Rc::clone(&create_host_jump_chain);
        hb.on_add_create_host_jump_hop(move |id, name, address, port| {
            if let Some(w) = window_weak.upgrade() {
                let mut list = chain_add.borrow_mut();
                let addr_str = address.to_string().trim().to_string();
                if !addr_str.is_empty() {
                    let name_str = name.to_string().trim().to_string();
                    let final_name = if name_str.is_empty() { addr_str.clone() } else { name_str };
                    list.push(JumpHopItemData {
                        id: id.clone(),
                        name: final_name.into(),
                        address: addr_str.into(),
                        port: if port <= 0 { 22 } else { port },
                    });
                    sync_hosts_bridge_create_host_jump_chain(&w, &list);
                }
            }
        });
    }
    {
        let window_weak = window.as_weak();
        let chain_import = Rc::clone(&create_host_jump_chain);
        hb.on_import_create_host_preset_chain(move |hops_serialized, replace| {
            if let Some(w) = window_weak.upgrade() {
                let mut list = chain_import.borrow_mut();
                if replace {
                    list.clear();
                }
                for part in hops_serialized.split(';') {
                    let segs: Vec<&str> = part.split(',').collect();
                    if segs.len() >= 4 {
                        let p: i32 = segs[3].parse().unwrap_or(22);
                        list.push(JumpHopItemData {
                            id: segs[0].into(),
                            name: segs[1].into(),
                            address: segs[2].into(),
                            port: p,
                        });
                    }
                }
                sync_hosts_bridge_create_host_jump_chain(&w, &list);
            }
        });
    }
    {
        let window_weak = window.as_weak();
        let chain_move = Rc::clone(&create_host_jump_chain);
        hb.on_move_create_host_jump_hop(move |from_idx, to_idx| {
            if let Some(w) = window_weak.upgrade() {
                let mut list = chain_move.borrow_mut();
                let f = from_idx as usize;
                let t = to_idx as usize;
                if f < list.len() && t < list.len() {
                    list.swap(f, t);
                    sync_hosts_bridge_create_host_jump_chain(&w, &list);
                }
            }
        });
    }
    {
        let window_weak = window.as_weak();
        let chain_rm = Rc::clone(&create_host_jump_chain);
        hb.on_remove_create_host_jump_hop(move |idx| {
            if let Some(w) = window_weak.upgrade() {
                let mut list = chain_rm.borrow_mut();
                let i = idx as usize;
                if i < list.len() {
                    list.remove(i);
                    sync_hosts_bridge_create_host_jump_chain(&w, &list);
                }
            }
        });
    }
    {
        let window_weak = window.as_weak();
        let chain_clr = Rc::clone(&create_host_jump_chain);
        hb.on_clear_create_host_jump_chain(move || {
            if let Some(w) = window_weak.upgrade() {
                chain_clr.borrow_mut().clear();
                sync_hosts_bridge_create_host_jump_chain(&w, &[]);
            }
        });
    }

    // -------------------------------------------------------------------------
    // 11. 关闭新建主机模态弹窗
    // -------------------------------------------------------------------------
    {
        let window_weak = window.as_weak();
        hb.on_close_create_host_modal(move || {
            if let Some(w) = window_weak.upgrade() {
                let hb = w.global::<HostsBridge>();
                hb.set_is_create_host_modal_open(false);
                hb.set_is_edit_mode(false);
                hb.set_editing_host_id("".into());
            }
        });
    }

    // -------------------------------------------------------------------------
    // 11.1 凭据筛选过滤回调 (按分类与关键字动态检索凭据库)
    // -------------------------------------------------------------------------
    {
        let window_weak_filter_cred = window.as_weak();
        let core_state_filter_cred = Rc::clone(&ctx.core_state);
        hb.on_filter_create_host_credentials(move |category, query| {
            if let Some(w) = window_weak_filter_cred.upgrade() {
                let cat_str = category.to_string().trim().to_lowercase();
                let q_str = query.to_string().trim().to_lowercase();

                if let Ok(creds) = core_state_filter_cred.storage().credentials().list_all() {
                    let filtered: Vec<CredentialOptionData> = creds
                        .into_iter()
                        .filter(|c| {
                            // 1. 分类匹配
                            let type_str = c.cred_type.to_string().to_lowercase();
                            let matches_cat = match cat_str.as_str() {
                                "" | "all" => true,
                                "key" => type_str == "key",
                                "password" => type_str == "password",
                                "agent" => type_str == "agent",
                                _ => true,
                            };
                            if !matches_cat {
                                return false;
                            }

                            // 2. 搜索关键字匹配 (名称、用户名、算法、ID)
                            if q_str.is_empty() {
                                true
                            } else {
                                c.name.to_lowercase().contains(&q_str)
                                    || c.username.as_deref().unwrap_or("").to_lowercase().contains(&q_str)
                                    || c.algorithm.to_lowercase().contains(&q_str)
                                    || c.id.to_lowercase().contains(&q_str)
                            }
                        })
                        .map(|c| CredentialOptionData {
                            id: c.id.into(),
                            name: c.name.into(),
                            username: c.username.unwrap_or_else(|| "root".to_string()).into(),
                            cred_type: c.cred_type.to_string().into(),
                            algorithm: c.algorithm.into(),
                        })
                        .collect();

                    sync_hosts_bridge_credentials(&w, &filtered);
                    tracing::debug!(target: "smagical_ui::hosts", "新建主机弹窗凭据检索: 分类='{}', 关键字='{}', 匹配条数={}", cat_str, q_str, filtered.len());
                }
            }
        });
    }

    // -------------------------------------------------------------------------
    // 11.2 代理筛选过滤回调 (按关键字动态检索隧道中心代理通道)
    // -------------------------------------------------------------------------
    {
        let window_weak_filter_proxy = window.as_weak();
        let core_state_filter_proxy = Rc::clone(&ctx.core_state);
        hb.on_filter_create_host_proxies(move |query| {
            if let Some(w) = window_weak_filter_proxy.upgrade() {
                let q_str = query.to_string().trim().to_lowercase();
                if let Ok(tunnels) = core_state_filter_proxy.storage().tunnels().list_all() {
                    let filtered: Vec<ProxyOptionData> = tunnels
                        .into_iter()
                        .filter(|t| t.tunnel_type == smagical_core::TunnelType::ProxyServer || t.tunnel_type == smagical_core::TunnelType::Dynamic)
                        .filter_map(|t| {
                            let proto = if t.tunnel_type == smagical_core::TunnelType::Dynamic {
                                "SOCKS5".to_string()
                            } else if !t.proxy_proto.is_empty() {
                                t.proxy_proto.to_uppercase()
                            } else {
                                "SOCKS5".to_string()
                            };
                            let (host, port) = if t.tunnel_type == smagical_core::TunnelType::Dynamic {
                                let h = if !t.local_bind.is_empty() { t.local_bind.clone() } else { "127.0.0.1".to_string() };
                                (h, t.local_port as i32)
                            } else {
                                let h = if !t.remote_host.is_empty() { t.remote_host.clone() } else if !t.local_bind.is_empty() { t.local_bind.clone() } else { "127.0.0.1".to_string() };
                                let p = if t.remote_port > 0 { t.remote_port as i32 } else { t.local_port as i32 };
                                (h, p)
                            };
                            let matches = if q_str.is_empty() {
                                true
                            } else {
                                t.name.to_lowercase().contains(&q_str)
                                    || proto.to_lowercase().contains(&q_str)
                                    || host.to_lowercase().contains(&q_str)
                                    || port.to_string().contains(&q_str)
                                    || t.proxy_username.to_lowercase().contains(&q_str)
                            };
                            if matches {
                                Some(ProxyOptionData {
                                    id: t.id.clone().into(),
                                    name: t.name.clone().into(),
                                    proto: proto.into(),
                                    host: host.into(),
                                    port,
                                    username: t.proxy_username.clone().into(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                    sync_hosts_bridge_proxies(&w, &filtered);
                    tracing::debug!(target: "smagical_ui::hosts", "新建主机弹窗代理检索: 关键字='{}', 匹配条数={}", q_str, filtered.len());
                }
            }
        });
    }

    // -------------------------------------------------------------------------
    // 12. 异步测试主机 TCP 端口连通性与 RTT 测速
    // -------------------------------------------------------------------------
    {
        let window_weak_test = window.as_weak();
        hb.on_test_host_connection(move |address, port| {
            if let Some(w) = window_weak_test.upgrade() {
                let addr_str = address.to_string().trim().to_string();
                let port_num = if port <= 0 || port > 65535 { 22 } else { port as u16 };

                let is_en = w.global::<WindowBridge>().get_current_language() == "en-US";

                if addr_str.is_empty() {
                    w.global::<HostsBridge>().set_test_connection_status("error".into());
                    w.global::<HostsBridge>().set_test_connection_message(
                        if is_en { "Host address cannot be empty".into() } else { "主机地址不能为空".into() }
                    );
                    return;
                }

                w.global::<HostsBridge>().set_test_connection_status("testing".into());
                w.global::<HostsBridge>().set_test_connection_message(
                    if is_en { "Probing TCP handshake...".into() } else { "正在发起 TCP 握手探活...".into() }
                );

                let window_weak_bg = window_weak_test.clone();
                std::thread::Builder::new()
                    .name("smalux-tcp-ping".to_string())
                    .spawn(move || {
                        let target = format!("{}:{}", addr_str, port_num);
                        let start = std::time::Instant::now();
                        let timeout = std::time::Duration::from_secs(5);

                        let result = match target.to_socket_addrs() {
                            Ok(mut addrs) => {
                                if let Some(sock_addr) = addrs.next() {
                                    std::net::TcpStream::connect_timeout(&sock_addr, timeout)
                                } else {
                                    let not_found_msg = if is_en { "Unable to resolve host address" } else { "无法解析域名地址" };
                                    Err(std::io::Error::new(std::io::ErrorKind::NotFound, not_found_msg))
                                }
                            }
                            Err(e) => Err(e),
                        };

                        let elapsed = start.elapsed().as_millis();

                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(win) = window_weak_bg.upgrade() {
                                let bridge = win.global::<HostsBridge>();
                                match result {
                                    Ok(_stream) => {
                                        bridge.set_test_connection_status("success".into());
                                        let msg = if is_en {
                                            format!("Connection successful ({}ms)", elapsed)
                                        } else {
                                            format!("连接成功 ({}ms)", elapsed)
                                        };
                                        bridge.set_test_connection_message(msg.into());
                                    }
                                    Err(err) => {
                                        bridge.set_test_connection_status("error".into());
                                        let msg = if is_en {
                                            format!("Handshake failed: {}", err)
                                        } else {
                                            format!("握手失败: {}", err)
                                        };
                                        bridge.set_test_connection_message(msg.into());
                                    }
                                }
                            }
                        });
                    })
                    .ok();
            }
        });
    }

    // -------------------------------------------------------------------------
    // 13. 提交新建主机配置并同步持久化至存储层
    // -------------------------------------------------------------------------
    {
        let window_weak_submit = window.as_weak();
        let core_state_submit = Rc::clone(&ctx.core_state);
        let master_tree_submit = Rc::clone(&ctx.master_tree);
        let master_cards_submit = Rc::clone(&ctx.master_cards);
        let expanded_submit = Rc::clone(&ctx.expanded_groups);
        let selector_expanded_submit = Rc::clone(&ctx.selector_expanded_groups);
        let notifications_submit = ctx.notifications.clone();

        hb.on_submit_create_host(move |
            name,
            address,
            port,
            parent_group_id,
            auth_type,
            credential_id,
            username,
            password,
            key_data,
            key_passphrase,
            save_to_credentials,
            proxy_type,
            proxy_host,
            proxy_port,
            proxy_username,
            proxy_password,
            jump_chain_summary,
            keepalive_interval,
            initial_dir,
            startup_cmd,
            term_type,
            notes,
        | {
            if let Some(w) = window_weak_submit.upgrade() {
                let addr_str = address.to_string().trim().to_string();
                if addr_str.is_empty() {
                    notifications_submit.error("保存失败", "主机地址不能为空");
                    return;
                }

                let name_str = name.to_string().trim().to_string();
                let final_name = if name_str.is_empty() { addr_str.clone() } else { name_str };
                let port_u16 = if port <= 0 || port > 65535 { 22 } else { port as u16 };
                let parent_id_str = parent_group_id.to_string();
                let final_parent_id = if parent_id_str.is_empty() || parent_id_str == "root" {
                    None
                } else {
                    Some(parent_id_str.clone())
                };

                let hb = w.global::<HostsBridge>();
                let is_edit = hb.get_is_edit_mode();
                let edit_id = hb.get_editing_host_id().to_string();
                let target_host_id = if is_edit && !edit_id.is_empty() {
                    edit_id.clone()
                } else {
                    format!("host-{}", uuid::Uuid::new_v4())
                };

                let (existing_sort, existing_status, existing_ping) = if is_edit {
                    if let Ok(Some(orig)) = core_state_submit.storage().hosts().get_by_id(&target_host_id) {
                        (orig.sort_order, orig.status, orig.ping_ms)
                    } else {
                        (0, smagical_core::HostStatus::Online, 0)
                    }
                } else {
                    (0, smagical_core::HostStatus::Online, 0)
                };

                // 处理身份认证信息与凭据关联
                let auth_type_str = auth_type.to_string();
                let user_str = username.to_string().trim().to_string();
                let pwd_str = password.to_string();
                let key_str = key_data.to_string();
                let pass_str = key_passphrase.to_string();

                let mut final_cred_id: Option<String> = None;
                let final_username = if user_str.is_empty() { None } else { Some(user_str.clone()) };
                let final_password = if pwd_str.is_empty() { None } else { Some(pwd_str.clone()) };
                let final_key_data = if key_str.is_empty() { None } else { Some(key_str.clone()) };
                let final_key_pass = if pass_str.is_empty() { None } else { Some(pass_str.clone()) };

                if auth_type_str == "credential" {
                    let cid = credential_id.to_string();
                    if !cid.is_empty() {
                        final_cred_id = Some(cid);
                    }
                } else if auth_type_str == "password" {
                    if save_to_credentials && (!pwd_str.is_empty() || !user_str.is_empty()) {
                        let cred_id = format!("cred-{}", uuid::Uuid::new_v4());
                        let cred_rec = smagical_core::CredentialRecord {
                            id: cred_id.clone(),
                            name: format!("{}-密码凭据", final_name),
                            cred_type: smagical_core::CredentialType::Password,
                            algorithm: "Password".to_string(),
                            username: final_username.clone(),
                            secret_data: pwd_str,
                            passphrase: None,
                            public_key: None,
                            fingerprint: None,
                            bound_host_count: 1,
                            created_at: "刚刚".to_string(),
                            updated_at: "刚刚".to_string(),
                            notes: format!("关联主机 [{}]", final_name),
                        };
                        let _ = core_state_submit.storage().credentials().save(&cred_rec);
                        final_cred_id = Some(cred_id);
                    }
                } else if auth_type_str == "key" {
                    if save_to_credentials && !key_str.is_empty() {
                        let cred_id = format!("cred-{}", uuid::Uuid::new_v4());
                        let cred_rec = smagical_core::CredentialRecord {
                            id: cred_id.clone(),
                            name: format!("{}-私钥凭据", final_name),
                            cred_type: smagical_core::CredentialType::Key,
                            algorithm: "SSH Key".to_string(),
                            username: final_username.clone(),
                            secret_data: key_str,
                            passphrase: final_key_pass.clone(),
                            public_key: None,
                            fingerprint: None,
                            bound_host_count: 1,
                            created_at: "刚刚".to_string(),
                            updated_at: "刚刚".to_string(),
                            notes: format!("关联主机 [{}]", final_name),
                        };
                        let _ = core_state_submit.storage().credentials().save(&cred_rec);
                        final_cred_id = Some(cred_id);
                    }
                }

                // 处理代理参数
                let ptype_str = proxy_type.to_string().trim().to_string();
                let (final_proxy_type, final_proxy_host, final_proxy_port, final_proxy_user, final_proxy_pass) =
                    if ptype_str.is_empty() || ptype_str == "direct" || ptype_str == "none" {
                        (None, None, None, None, None)
                    } else {
                        let phost = proxy_host.to_string().trim().to_string();
                        let puser = proxy_username.to_string().trim().to_string();
                        let ppass = proxy_password.to_string();
                        (
                            Some(ptype_str),
                            if phost.is_empty() { None } else { Some(phost) },
                            if proxy_port > 0 && proxy_port <= 65535 { Some(proxy_port as u16) } else { None },
                            if puser.is_empty() { None } else { Some(puser) },
                            if ppass.is_empty() { None } else { Some(ppass) },
                        )
                    };

                // 处理跳板机链路
                let jsummary_str = jump_chain_summary.to_string().trim().to_string();
                let final_jump_chain = if jsummary_str.is_empty() { None } else { Some(jsummary_str) };

                // 处理高级选项
                let keepalive_val = if keepalive_interval <= 0 { 30 } else { keepalive_interval as u32 };
                let idir_str = initial_dir.to_string().trim().to_string();
                let final_initial_dir = if idir_str.is_empty() { None } else { Some(idir_str) };
                let cmd_str = startup_cmd.to_string().trim().to_string();
                let final_startup_cmd = if cmd_str.is_empty() { None } else { Some(cmd_str) };
                let ttype_str = term_type.to_string().trim().to_string();
                let final_term_type = if ttype_str.is_empty() { Some("xterm-256color".to_string()) } else { Some(ttype_str) };

                // 写入持久化存储引擎 (MockStorage)
                let host_rec = smagical_core::HostRecord {
                    id: target_host_id.clone(),
                    name: final_name.clone(),
                    address: addr_str.clone(),
                    port: port_u16,
                    parent_group_id: final_parent_id.clone(),
                    credential_id: final_cred_id.clone(),
                    status: existing_status,
                    ping_ms: existing_ping,
                    sort_order: existing_sort,
                    notes: notes.to_string(),

                    auth_type: auth_type_str,
                    username: final_username,
                    password: final_password,
                    key_data: final_key_data,
                    key_passphrase: final_key_pass,

                    proxy_type: final_proxy_type,
                    proxy_host: final_proxy_host,
                    proxy_port: final_proxy_port,
                    proxy_username: final_proxy_user,
                    proxy_password: final_proxy_pass,

                    jump_chain_summary: final_jump_chain,

                    keepalive_interval: keepalive_val,
                    connect_timeout: 10,
                    initial_dir: final_initial_dir,
                    startup_cmd: final_startup_cmd,
                    term_type: final_term_type,
                };

                if let Err(e) = core_state_submit.storage().hosts().save(&host_rec) {
                    notifications_submit.error("保存失败", format!("写入存储层受限: {}", e));
                    return;
                }

                // 派发 HostAssetChangedEvent 事件
                core_state_submit.events().dispatch(&HostAssetChangedEvent {
                    host_id: target_host_id.clone(),
                    name: final_name.clone(),
                    address: addr_str.clone(),
                    credential_id: final_cred_id,
                    action: if is_edit { "updated".to_string() } else { "created".to_string() },
                });

                // 从数据层（SSOT）重新完整构建整棵树与各分组子项计数 (item_count)
                let new_raw_tree = build_raw_tree_from_storage(core_state_submit.storage().as_ref());
                *master_tree_submit.borrow_mut() = new_raw_tree;

                // 如果属于某个分组，自动将该目标分组及其祖先加入展开集合，保证新建主机立即可见
                if let Some(ref pid) = final_parent_id {
                    let mut exp = expanded_submit.borrow_mut();
                    let mut curr = pid.clone();
                    let tree = master_tree_submit.borrow();
                    while !curr.is_empty() {
                        exp.insert(curr.clone());
                        if let Some(p) = tree.iter().find(|n| n.id == curr) {
                            curr = p.parent_id.clone();
                        } else {
                            break;
                        }
                    }
                }

                // 基于最新数据层同步更新平铺卡片列表
                let all_hosts = core_state_submit.storage().hosts().list_all().unwrap_or_default();
                let all_groups = core_state_submit.storage().groups().list_all().unwrap_or_default();
                let new_cards: Vec<HostItemData> = all_hosts.iter().map(|h| {
                    let g_name = if let Some(ref pid) = h.parent_group_id {
                        all_groups.iter().find(|g| g.id == *pid).map(|g| g.name.clone()).unwrap_or_else(|| "根目录".to_string())
                    } else {
                        "根目录".to_string()
                    };
                    HostItemData {
                        id: h.id.clone().into(),
                        name: h.name.clone().into(),
                        address: h.address.clone().into(),
                        port: h.port as i32,
                        group: g_name.into(),
                        status: h.status.to_string().into(),
                        ping_ms: h.ping_ms,
                    }
                }).collect();
                *master_cards_submit.borrow_mut() = new_cards;

                // 同步刷新 UI 树形视图、卡片列表与选择器下拉选项
                let tree = master_tree_submit.borrow();
                let expanded_set = expanded_submit.borrow();
                let selector_set = selector_expanded_submit.borrow();

                let visible_nodes = build_visible_tree_nodes(&tree, &expanded_set);
                sync_hosts_bridge_tree(&w, &visible_nodes);

                let cards = master_cards_submit.borrow();
                sync_hosts_bridge_cards(&w, &cards);

                let group_options = build_group_options(&tree, &selector_set);
                sync_hosts_bridge_options(&w, &group_options);

                // 关闭弹窗并重置编辑状态，推送成功全局气泡通知
                let hb = w.global::<HostsBridge>();
                hb.set_is_create_host_modal_open(false);
                hb.set_is_edit_mode(false);
                hb.set_editing_host_id("".into());
                if is_edit {
                    notifications_submit.success("修改主机成功", format!("主机 [{}] 配置已成功更新", final_name));
                    tracing::info!(target: "smagical_ui::hosts", "成功修改主机配置: {} ({})", final_name, target_host_id);
                } else {
                    notifications_submit.success("创建主机成功", format!("主机 [{}] 已成功保存至资产列表", final_name));
                    tracing::info!(target: "smagical_ui::hosts", "成功创建新主机记录: {} ({})", final_name, target_host_id);
                }
            }
        });
    }

    // -------------------------------------------------------------------------
    // 14. 右键菜单 - 快捷打开新建分组弹窗并预设父级分组
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    hb.on_open_create_group_modal(move |parent_id| {
        if let Some(w) = window_weak.upgrade() {
            let hb = w.global::<HostsBridge>();
            hb.set_create_group_default_parent(parent_id);
            hb.set_is_create_group_open(true);
        }
    });

    // -------------------------------------------------------------------------
    // 15. 右键菜单 - 复制主机地址或命令至剪贴板
    // -------------------------------------------------------------------------
    let notifications_clip = ctx.notifications.clone();
    hb.on_copy_to_clipboard(move |text| {
        let text_str = text.to_string();
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(text_str.clone());
            notifications_clip.success("已复制到剪贴板", text_str);
        }
    });

    // -------------------------------------------------------------------------
    // 16. 右键菜单 - 打开远程 SFTP 文件传输
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let notifications_files = ctx.notifications.clone();
    hb.on_open_host_files(move |host_id| {
        if let Some(w) = window_weak.upgrade() {
            let wb = w.global::<WindowBridge>();
            wb.set_main_view("files".into());
            wb.set_active_left_tab("files".into());
            wb.set_is_left_drawer_open(true);
            let fb = w.global::<FilesBridge>();
            fb.invoke_open_host_files(host_id.clone());
            notifications_files.info("打开 SFTP", format!("正在载入主机 [{}] 的文件管理器...", host_id));
        }
    });

    // -------------------------------------------------------------------------
    // 17. 右键菜单 - 克隆主机记录
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let master_tree_clone = Rc::clone(&ctx.master_tree);
    let master_cards_clone = Rc::clone(&ctx.master_cards);
    let expanded_clone = Rc::clone(&ctx.expanded_groups);
    let selector_expanded_clone = Rc::clone(&ctx.selector_expanded_groups);
    let core_state_clone = Rc::clone(&ctx.core_state);
    let notifications_clone = ctx.notifications.clone();
    hb.on_clone_host(move |host_id| {
        if let Some(w) = window_weak.upgrade() {
            let hid = host_id.to_string();
            let storage = core_state_clone.storage();
            if let Ok(Some(orig)) = storage.hosts().get_by_id(&hid) {
                let new_id = format!("host-{}", uuid::Uuid::new_v4().simple());
                let cloned_name = format!("{} (副本)", orig.name);
                let mut cloned_rec = orig.clone();
                cloned_rec.id = new_id.clone();
                cloned_rec.name = cloned_name.clone();
                cloned_rec.sort_order += 1;
                if let Err(e) = storage.hosts().save(&cloned_rec) {
                    notifications_clone.error("克隆失败", format!("保存至数据层受限: {}", e));
                    return;
                }

                core_state_clone.events().dispatch(&HostAssetChangedEvent {
                    host_id: new_id.clone(),
                    name: cloned_name.clone(),
                    address: cloned_rec.address.clone(),
                    credential_id: None,
                    action: "cloned".to_string(),
                });

                // 重新同步树与卡片
                let new_raw_tree = build_raw_tree_from_storage(storage.as_ref());
                *master_tree_clone.borrow_mut() = new_raw_tree;

                let all_hosts = storage.hosts().list_all().unwrap_or_default();
                let all_groups = storage.groups().list_all().unwrap_or_default();
                let new_cards: Vec<HostItemData> = all_hosts.iter().map(|h| {
                    let g_name = if let Some(ref pid) = h.parent_group_id {
                        all_groups.iter().find(|g| g.id == *pid).map(|g| g.name.clone()).unwrap_or_else(|| "根目录".to_string())
                    } else {
                        "根目录".to_string()
                    };
                    HostItemData {
                        id: h.id.clone().into(),
                        name: h.name.clone().into(),
                        address: h.address.clone().into(),
                        port: h.port as i32,
                        group: g_name.into(),
                        status: h.status.to_string().into(),
                        ping_ms: h.ping_ms,
                    }
                }).collect();
                *master_cards_clone.borrow_mut() = new_cards;

                let tree = master_tree_clone.borrow();
                let exp = expanded_clone.borrow();
                let sel = selector_expanded_clone.borrow();
                let visible = build_visible_tree_nodes(&tree, &exp);
                sync_hosts_bridge_tree(&w, &visible);
                let cards = master_cards_clone.borrow();
                sync_hosts_bridge_cards(&w, &cards);
                let options = build_group_options(&tree, &sel);
                sync_hosts_bridge_options(&w, &options);

                notifications_clone.success("克隆主机成功", format!("主机 [{}] 已成功创建副本", cloned_name));
            }
        }
    });

    // -------------------------------------------------------------------------
    // 18. 右键菜单 - 删除主机或分组节点
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let master_tree_del = Rc::clone(&ctx.master_tree);
    let master_cards_del = Rc::clone(&ctx.master_cards);
    let expanded_del = Rc::clone(&ctx.expanded_groups);
    let selector_expanded_del = Rc::clone(&ctx.selector_expanded_groups);
    let core_state_del = Rc::clone(&ctx.core_state);
    let notifications_del = ctx.notifications.clone();
    hb.on_delete_node(move |target_id, is_group| {
        if let Some(w) = window_weak.upgrade() {
            let tid = target_id.to_string();
            let storage = core_state_del.storage();
            if is_group {
                // 删除分组前，将其直属主机移入根目录，避免数据孤儿
                if let Ok(hosts) = storage.hosts().list_all() {
                    for mut h in hosts {
                        if h.parent_group_id.as_deref() == Some(&tid) {
                            h.parent_group_id = None;
                            let _ = storage.hosts().save(&h);
                        }
                    }
                }
                let _ = storage.groups().delete(&tid);
                expanded_del.borrow_mut().remove(&tid);
                selector_expanded_del.borrow_mut().remove(&tid);

                notifications_del.success("删除成功", "分组已删除，内部主机已安全移至根目录");
            } else {
                let _ = storage.hosts().delete(&tid);
                notifications_del.success("删除成功", "目标主机资产已从列表中移除");
            }

            core_state_del.events().dispatch(&HostAssetChangedEvent {
                host_id: tid.clone(),
                name: "".to_string(),
                address: "".to_string(),
                credential_id: None,
                action: "deleted".to_string(),
            });

            // 重新同步树与卡片
            let new_raw_tree = build_raw_tree_from_storage(storage.as_ref());
            *master_tree_del.borrow_mut() = new_raw_tree;

            let all_hosts = storage.hosts().list_all().unwrap_or_default();
            let all_groups = storage.groups().list_all().unwrap_or_default();
            let new_cards: Vec<HostItemData> = all_hosts.iter().map(|h| {
                let g_name = if let Some(ref pid) = h.parent_group_id {
                    all_groups.iter().find(|g| g.id == *pid).map(|g| g.name.clone()).unwrap_or_else(|| "根目录".to_string())
                } else {
                    "根目录".to_string()
                };
                HostItemData {
                    id: h.id.clone().into(),
                    name: h.name.clone().into(),
                    address: h.address.clone().into(),
                    port: h.port as i32,
                    group: g_name.into(),
                    status: h.status.to_string().into(),
                    ping_ms: h.ping_ms,
                }
            }).collect();
            *master_cards_del.borrow_mut() = new_cards;

            let tree = master_tree_del.borrow();
            let exp = expanded_del.borrow();
            let sel = selector_expanded_del.borrow();
            let visible = build_visible_tree_nodes(&tree, &exp);
            sync_hosts_bridge_tree(&w, &visible);
            let cards = master_cards_del.borrow();
            sync_hosts_bridge_cards(&w, &cards);
            let options = build_group_options(&tree, &sel);
            sync_hosts_bridge_options(&w, &options);
        }
    });
}
