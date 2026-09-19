//! 主机与分组资产管理、树形/列表视图拖拽移动、快速打开终端等交互回调绑定。
//!
//! 包含多级分组嵌套维护、无环拓扑防呆校验、平滑调序与动态视口宽度计算。

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use slint::{ComponentHandle, Model};
use smagical_core::event::{
    HostAssetChangedEvent, HostGroupToggledEvent, HostTreeReorderedEvent, TerminalSessionEvent,
};
use smagical_core::{AppStorage, CredentialRecord, CredentialType, GroupRecord, HostRecord, HostStatus};
use crate::async_util::spawn_async;
use crate::store::diff::{compute_card_diff, compute_tree_diff};

use crate::generated::{
    AppWindow, CredentialOptionData, FilesBridge, GroupOptionData, HostItemData, HostTreeNode, HostsBridge,
    JumpHostOptionData, JumpHopItemData, PresetJumpChainData, ProxyOptionData, WindowBridge,
};
use crate::handlers::AppContext;
use crate::session::{sync_active_session_ui, TerminalSessionInfo};
use crate::terminal::TerminalInstance;
use crate::tree_model::{
    build_cards_from_records, build_group_options, build_raw_tree, build_search_tree_nodes,
    build_visible_tree_nodes, calculate_max_tree_width, move_and_reorder_raw_node, RawTreeNode,
};

fn sync_hosts_bridge_tree(w: &AppWindow, nodes: &[HostTreeNode]) {
    let hb = w.global::<HostsBridge>();
    let current_tree = hb.get_tree_nodes();
    let current_count = current_tree.row_count();
    let mut old_nodes = Vec::with_capacity(current_count);
    for i in 0..current_count {
        if let Some(n) = current_tree.row_data(i) {
            old_nodes.push(n);
        }
    }

    let diffs = compute_tree_diff(&old_nodes, nodes);
    if diffs.is_empty() {
        return;
    }

    let model = slint::ModelRc::from(Rc::new(slint::VecModel::from(nodes.to_vec())));
    let width = calculate_max_tree_width(nodes);
    hb.set_tree_nodes(model);
    hb.set_tree_content_width(width);
}

fn sync_hosts_bridge_cards(w: &AppWindow, cards: &[HostItemData]) {
    let hb = w.global::<HostsBridge>();
    let current_cards = hb.get_hosts();
    let current_count = current_cards.row_count();
    let mut old_cards = Vec::with_capacity(current_count);
    for i in 0..current_count {
        if let Some(c) = current_cards.row_data(i) {
            old_cards.push(c);
        }
    }

    let diffs = compute_card_diff(&old_cards, cards);
    if diffs.is_empty() {
        return;
    }

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

#[allow(dead_code)]
fn render_hosts_ui(
    w: &AppWindow,
    tree: &[RawTreeNode],
    cards: &[HostItemData],
    expanded: &HashSet<String>,
    selector_expanded: &HashSet<String>,
    search_query: &str,
) {
    let q = search_query.trim();
    let visible_nodes = if q.is_empty() {
        build_visible_tree_nodes(tree, expanded)
    } else {
        build_search_tree_nodes(tree, q)
    };
    sync_hosts_bridge_tree(w, &visible_nodes);

    let display_cards: Vec<HostItemData> = if q.is_empty() {
        cards.to_vec()
    } else {
        let q_lower = q.to_lowercase();
        cards.iter().filter(|h| {
            h.name.to_lowercase().contains(&q_lower)
                || h.address.to_lowercase().contains(&q_lower)
                || h.group.to_lowercase().contains(&q_lower)
        }).cloned().collect()
    };
    sync_hosts_bridge_cards(w, &display_cards);

    let group_options = build_group_options(tree, selector_expanded);
    sync_hosts_bridge_options(w, &group_options);
}

async fn sync_ui_hosts_async(
    storage: &dyn AppStorage,
    master_tree: &Arc<RwLock<Vec<RawTreeNode>>>,
    master_cards: &Arc<RwLock<Vec<HostItemData>>>,
    expanded: &Arc<RwLock<HashSet<String>>>,
    selector_expanded: &Arc<RwLock<HashSet<String>>>,
    search_query: &Arc<RwLock<String>>,
    window_weak: slint::Weak<AppWindow>,
) {
    let groups = storage.groups().list_all().await.unwrap_or_default();
    let hosts = storage.hosts().list_all().await.unwrap_or_default();
    let credentials = storage.credentials().list_all().await.unwrap_or_default();

    let new_tree = build_raw_tree(&groups, &hosts, &credentials);
    let new_cards = build_cards_from_records(&hosts, &groups);

    *master_tree.write().unwrap() = new_tree.clone();
    *master_cards.write().unwrap() = new_cards.clone();

    let exp = expanded.read().unwrap().clone();
    let sel = selector_expanded.read().unwrap().clone();
    let q = search_query.read().unwrap().clone();

    let _ = slint::invoke_from_event_loop(move || {
        if let Some(w) = window_weak.upgrade() {
            render_hosts_ui(&w, &new_tree, &new_cards, &exp, &sel, &q);
        }
    });
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
    let host_store_selector = Arc::clone(&ctx.host_store);
    hb.on_toggle_selector_group(move |id| {
        if let Some(w) = window_weak.upgrade() {
            host_store_selector.toggle_selector_group(&id);
            let tree = host_store_selector.master_tree.read().unwrap();
            let set = host_store_selector.selector_expanded_groups.read().unwrap();
            let next_options = build_group_options(&tree, &set);
            sync_hosts_bridge_options(&w, &next_options);
        }
    });

    // -------------------------------------------------------------------------
    // 2. 侧边栏树形结构分组折叠 / 展开回调 (优化点 1 & 2：集中式 Store 管理 + 后台并发更新)
    // -------------------------------------------------------------------------
    // 点击左侧主机树中的某个文件夹节点时触发，切换展开状态并异步持久化至 AppStorage (0ms UI 阻塞)。
    let window_weak = window.as_weak();
    let host_store_toggle = Arc::clone(&ctx.host_store);
    let core_state_toggle = Rc::clone(&ctx.core_state);
    hb.on_toggle_group(move |id| {
        let id_str = id.to_string();
        let is_expanding = host_store_toggle.toggle_group(&id_str);

        // 异步持久化分组折叠/展开状态至存储层 (0ms 阻塞 UI 线程)
        let storage = core_state_toggle.storage();
        let id_for_storage = id_str.clone();
        spawn_async(async move {
            let _ = storage.groups().set_expanded(&id_for_storage, is_expanding).await;
        });

        // 显式派发分组折叠/展开事件
        core_state_toggle.events().dispatch(&HostGroupToggledEvent {
            group_id: id_str,
            is_expanded: is_expanding,
        });

        // 下沉至后台并发计算最新可见节点与字宽，并增量 Diff 更新界面
        host_store_toggle.schedule_tree_refresh(window_weak.clone());
    });

    // -------------------------------------------------------------------------
    // 3. 节点移动 / 拖拽层级调序回调
    // -------------------------------------------------------------------------
    // 鼠标拖拽松开后触发：支持树形层级物理迁移与卡片列表视觉调序双模式。
    let window_weak = window.as_weak();
    let master_tree_move = Arc::clone(&ctx.master_tree);
    let master_cards_move = Arc::clone(&ctx.master_cards);
    let expanded_move = Arc::clone(&ctx.expanded_groups);
    let selector_expanded_move = Arc::clone(&ctx.selector_expanded_groups);
    let search_query_move = Arc::clone(&ctx.search_query);
    let core_state_move = Rc::clone(&ctx.core_state);
    hb.on_move_node(move |src_id, target_id, drop_position| {
        if let Some(w) = window_weak.upgrade() {
            let src_str = src_id.to_string();
            let target_str = target_id.to_string();
            let pos_str = drop_position.to_string();
            let view_mode = w.global::<HostsBridge>().get_hosts_view_mode().to_string();

            // 1. 卡片平铺列表模式 (Card View Mode): 纯视觉显示排序调整，绝对锁定所属分组 (parent_id/group) 不变
            if view_mode == "card" {
                let mut cards = master_cards_move.write().unwrap();
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

                    // 异步同步列表排序至存储层 (0ms UI 阻塞)
                    let ordered_ids: Vec<String> = cards.iter().map(|c| c.id.to_string()).collect();
                    let storage = core_state_move.storage();
                    spawn_async(async move {
                        let _ = storage.hosts().update_list_order(&ordered_ids).await;
                    });

                    let q = search_query_move.read().unwrap().clone();
                    let display_cards: Vec<HostItemData> = if q.is_empty() {
                        cards.clone()
                    } else {
                        let q_lower = q.to_lowercase();
                        cards.iter().filter(|h| {
                            h.name.to_lowercase().contains(&q_lower)
                                || h.address.to_lowercase().contains(&q_lower)
                                || h.group.to_lowercase().contains(&q_lower)
                        }).cloned().collect()
                    };
                    sync_hosts_bridge_cards(&w, &display_cards);

                    tracing::info!(target: "smagical_ui::hosts", "成功调整列表模式主机展示顺序: [{}] 排在 [{}] 之后 (分组保持锁定，已异步同步存储层)", item_name, tgt_name);
                    core_state_move.events().dispatch(&HostTreeReorderedEvent {
                        source_id: src_str.clone(),
                        target_id: target_str.clone(),
                        position: pos_str.clone(),
                    });
                }
                return;
            }

            // 2. 树形层级模式 (Tree View Mode): 物理资产层级结构与文件夹迁移
            let mut tree = master_tree_move.write().unwrap();

            match move_and_reorder_raw_node(&mut tree, &src_str, &target_str, &pos_str) {
                Ok((src_name, target_name)) => {
                    // 如果移动到了具体分组内部，自动将该目标分组及其祖先加入展开集合
                    let mut exp = expanded_move.write().unwrap();
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
                    let q = search_query_move.read().unwrap().clone();
                    let next_nodes = if q.is_empty() {
                        build_visible_tree_nodes(&tree, &exp)
                    } else {
                        build_search_tree_nodes(&tree, &q)
                    };
                    sync_hosts_bridge_tree(&w, &next_nodes);

                    let next_options = build_group_options(&tree, &selector_expanded_move.read().unwrap());
                    sync_hosts_bridge_options(&w, &next_options);

                    // 异步同步树形结构迁移至存储层 (Host or Group) (0ms UI 阻塞)
                    if let Some(moved_node) = tree.iter().find(|n| n.id == src_str) {
                        let storage = core_state_move.storage();
                        let src_str_bg = src_str.clone();
                        let moved_is_group = moved_node.is_group;
                        let moved_parent_id = if moved_node.parent_id.is_empty() { None } else { Some(moved_node.parent_id.clone()) };
                        spawn_async(async move {
                            if moved_is_group {
                                let _ = storage.groups().move_group(&src_str_bg, moved_parent_id.as_deref()).await;
                            } else if let Ok(Some(mut host_rec)) = storage.hosts().get_by_id(&src_str_bg).await {
                                host_rec.parent_group_id = moved_parent_id;
                                let _ = storage.hosts().save(&host_rec).await;
                            }
                        });
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

                    let mut cards = master_cards_move.write().unwrap();
                    for card in cards.iter_mut() {
                        if card.id == src_str.as_str() {
                            card.group = new_group_name.clone().into();
                        }
                    }

                    let display_cards: Vec<HostItemData> = if q.is_empty() {
                        cards.clone()
                    } else {
                        let q_lower = q.to_lowercase();
                        cards.iter().filter(|h| {
                            h.name.to_lowercase().contains(&q_lower)
                                || h.address.to_lowercase().contains(&q_lower)
                                || h.group.to_lowercase().contains(&q_lower)
                        }).cloned().collect()
                    };
                    sync_hosts_bridge_cards(&w, &display_cards);

                    tracing::info!(target: "smagical_ui::hosts", "成功调序/移动树节点 [{}] (模式: {}, 目标: [{}], 已异步同步存储层)", src_name, pos_str, target_name);
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
    let master_tree_hover = Arc::clone(&ctx.master_tree);
    let master_cards_hover = Arc::clone(&ctx.master_cards);
    let expanded_hover = Arc::clone(&ctx.expanded_groups);
    let search_hover = Arc::clone(&ctx.search_query);
    hb.on_request_drag_hover(move |src_id, target_idx, _offset_in_row| {
        if let Some(w) = window_weak.upgrade() {
            let hb = w.global::<HostsBridge>();
            let src_str = src_id.to_string();
            let view_mode = hb.get_hosts_view_mode().to_string();

            // 1. 卡片模式悬停判定
            if view_mode == "card" {
                let cards = master_cards_hover.read().unwrap();
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
            let tree = master_tree_hover.read().unwrap();
            let q = search_hover.read().unwrap().clone();
            let visible_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_hover.read().unwrap())
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
    // 接收弹窗输入的分组名称与指定父级 ID，在树中创建分组并异步持久化到 AppStorage (0ms UI 阻塞)。
    let window_weak = window.as_weak();
    let master_tree_create = Arc::clone(&ctx.master_tree);
    let expanded_create = Arc::clone(&ctx.expanded_groups);
    let selector_expanded_create = Arc::clone(&ctx.selector_expanded_groups);
    let search_query_create = Arc::clone(&ctx.search_query);
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

            let mut tree = master_tree_create.write().unwrap();

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
                effective_username: None,
            };

            // 异步新增分组至底层存储层 (0ms UI 阻塞)
            let group_rec = if target_parent_id.is_empty() {
                GroupRecord::root(new_id.clone(), g_name.clone())
            } else {
                GroupRecord::child(new_id.clone(), g_name.clone(), target_parent_id.clone(), level)
            };
            let storage = core_state_create.storage();
            spawn_async(async move {
                let _ = storage.groups().save(&group_rec).await;
            });

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
                expanded_create.write().unwrap().insert(target_parent_id.clone());
                selector_expanded_create.write().unwrap().insert(target_parent_id);
            }
            // 新创建的分组自身默认展开
            expanded_create.write().unwrap().insert(new_id);

            tree.insert(insert_pos, new_group_node);

            // 刷新弹窗中的上级分组列表选项
            let next_options = build_group_options(&tree, &selector_expanded_create.read().unwrap());
            sync_hosts_bridge_options(&w, &next_options);

            // 刷新主界面树形结构
            let q = search_query_create.read().unwrap().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_create.read().unwrap())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            sync_hosts_bridge_tree(&w, &next_nodes);

            tracing::info!(target: "smagical_ui::tree", "创建新分组: {} (上级: {}, 已异步同步存储层)", g_name, if p_id.is_empty() { "根目录" } else { &p_id });
        }
    });

    // -------------------------------------------------------------------------
    // 6. 主机实时搜索过滤回调 (优化点 1 & 4：下沉至 Tokio 后台并发计算 + 增量比对 Diff，0ms 阻塞 UI)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let host_store_filter = Arc::clone(&ctx.host_store);
    let event_dispatcher_filter = ctx.core_state.event_manager().global().clone();
    hb.on_search_changed(move |query| {
        let q = query.trim().to_string();
        host_store_filter.schedule_search_compute(
            q,
            window_weak.clone(),
            Some(Arc::clone(&event_dispatcher_filter)),
        );
    });

    // -------------------------------------------------------------------------
    // 7. 打开主机终端会话回调
    // -------------------------------------------------------------------------
    // 8. 双击主机 / 本地 Shell 发起终端连接回调
    // -------------------------------------------------------------------------
    // 双击树形或卡片列表中的某个主机（或选择本地 Shell）时触发，分配会话 ID 并激活新 Tab。
    let window_weak = window.as_weak();
    let master_tree_open = Arc::clone(&ctx.master_tree);
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

                let spawn_res = TerminalInstance::spawn_local(sess_id.clone(), &h_id, session_name.clone(), 120, 32)
                    .or_else(|e| {
                        tracing::warn!(target: "smagical_ui::terminal", "启动指定 Shell [{}] 失败: {:?}，尝试回退系统 PowerShell...", h_id, e);
                        TerminalInstance::spawn_local(sess_id.clone(), "local-powershell", session_name.clone(), 120, 32)
                    })
                    .or_else(|e| {
                        tracing::warn!(target: "smagical_ui::terminal", "回退系统 PowerShell 失败: {:?}，尝试最后回退 CMD...", e);
                        TerminalInstance::spawn_local(sess_id.clone(), "local-cmd", session_name.clone(), 120, 32)
                    });

                match spawn_res {
                    Ok(instance) => {
                        active_terminals_open.borrow_mut().insert(sess_id.clone(), instance);
                    }
                    Err(err) => {
                        tracing::error!(target: "smagical_ui::terminal", "本地终端全部启动候选均失败: {:?}", err);
                        ctx_open.notify_error("终端启动失败", format!("无法创建本地终端进程: {}", err));
                        return;
                    }
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
                let tree = master_tree_open.read().unwrap();
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

                // 纯内存 0ms 获取有效用户名（优先使用凭据，无凭据或未设时回退使用主机直录账号）
                let username_opt = host_node.effective_username.clone();

                let instance_res = TerminalInstance::spawn_ssh(
                    sess_id.clone(),
                    session_name.clone(),
                    &host_node.address,
                    host_node.port as u16,
                    username_opt.as_deref(),
                    120,
                    32,
                )
                .or_else(|e| {
                    tracing::warn!(target: "smagical_ui::terminal", "SSH 会话启动失败: {:?}，回退本地终端...", e);
                    TerminalInstance::spawn_local(
                        sess_id.clone(),
                        "local-powershell",
                        session_name.clone(),
                        120,
                        32,
                    )
                    .or_else(|_| {
                        TerminalInstance::spawn_local(
                            sess_id.clone(),
                            "local-cmd",
                            session_name.clone(),
                            120,
                            32,
                        )
                    })
                });

                match instance_res {
                    Ok(instance) => {
                        active_terminals_open.borrow_mut().insert(sess_id.clone(), instance);
                    }
                    Err(err) => {
                        tracing::error!(target: "smagical_ui::terminal", "终端会话创建失败: {:?}", err);
                        ctx_open.notify_error("连接失败", format!("无法创建终端会话: {}", err));
                        return;
                    }
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
    let master_tree_open_host_modal = Arc::clone(&ctx.master_tree);
    let selector_expanded_open_host_modal = Arc::clone(&ctx.selector_expanded_groups);
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
            let tree = master_tree_open_host_modal.read().unwrap();
            let p_name = if p_id == "root" {
                "根目录 (顶级主机)".to_string()
            } else {
                tree.iter().find(|n| n.id == p_id && n.is_group).map(|n| n.name.clone()).unwrap_or_else(|| "根目录 (顶级主机)".to_string())
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

            // 2. 同步最新分组树形选项 (纯内存 0ms)
            let expanded_set = selector_expanded_open_host_modal.read().unwrap();
            let group_options = build_group_options(&tree, &expanded_set);
            sync_hosts_bridge_options(&w, &group_options);

            // 3. 从内存树同步可用跳板主机列表 (纯内存 0ms)
            let jump_options: Vec<JumpHostOptionData> = tree.iter()
                .filter(|n| !n.is_group)
                .map(|h| JumpHostOptionData {
                    id: h.id.clone().into(),
                    name: h.name.clone().into(),
                    address: h.address.clone().into(),
                    port: h.port,
                })
                .collect();
            sync_hosts_bridge_jump_hosts(&w, &jump_options);

            // 4. 异步拉取凭据库与网络隧道/代理配置 (0ms UI 阻塞)
            let storage = core_state_open_host_modal.storage();
            let w_weak = w.as_weak();
            spawn_async(async move {
                let creds = storage.credentials().list_all().await.unwrap_or_default();
                let tunnels = storage.tunnels().list_all().await.unwrap_or_default();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(win) = w_weak.upgrade() {
                        let cred_options: Vec<CredentialOptionData> = creds.into_iter().map(|c| {
                            CredentialOptionData {
                                id: c.id.into(),
                                name: c.name.into(),
                                username: c.username.unwrap_or_else(|| "root".to_string()).into(),
                                cred_type: c.cred_type.to_string().into(),
                                algorithm: c.algorithm.into(),
                            }
                        }).collect();
                        sync_hosts_bridge_credentials(&win, &cred_options);

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
                        sync_hosts_bridge_preset_jump_chains(&win, &preset_chains);

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
                        sync_hosts_bridge_proxies(&win, &proxies);
                    }
                });
            });

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
    let master_tree_edit_modal = Arc::clone(&ctx.master_tree);
    let selector_expanded_edit_modal = Arc::clone(&ctx.selector_expanded_groups);
    let create_host_jump_chain_edit = Rc::clone(&create_host_jump_chain);
    let notifications_edit_modal = ctx.notifications.clone();

    hb.on_open_edit_host_modal(move |host_id| {
        create_host_jump_chain_edit.borrow_mut().clear();
        let h_id = host_id.to_string();
        let storage = core_state_edit_modal.storage();
        let window_weak = window_weak_edit_modal.clone();
        let master_tree = Arc::clone(&master_tree_edit_modal);
        let selector_expanded = Arc::clone(&selector_expanded_edit_modal);
        let notifications = notifications_edit_modal.clone();

        spawn_async(async move {
            let host_opt = storage.hosts().get_by_id(&h_id).await.ok().flatten();
            let host = match host_opt {
                Some(h) => h,
                None => {
                    let _ = slint::invoke_from_event_loop(move || {
                        notifications.error("打开失败", format!("未找到主机 [{}] 的记录", h_id));
                    });
                    return;
                }
            };

            // 异步查询上级分组、关联凭据、跳板隧道、全量凭据/隧道 (0ms 阻塞 UI)
            let parent_name = if let Some(ref pid) = host.parent_group_id {
                storage.groups().get_by_id(pid).await.ok().flatten().map(|g| g.name).unwrap_or_else(|| "根目录 (顶级主机)".to_string())
            } else {
                "根目录 (顶级主机)".to_string()
            };

            let cred_info = if let Some(ref cid) = host.credential_id {
                if let Ok(Some(c)) = storage.credentials().get_by_id(cid).await {
                    (c.id, c.name, c.username.unwrap_or_else(|| "root".to_string()), c.cred_type.to_string(), c.algorithm)
                } else {
                    (cid.clone(), cid.clone(), "root".to_string(), "password".to_string(), "Password".to_string())
                }
            } else {
                (String::new(), String::new(), String::new(), String::new(), String::new())
            };

            let preset_tun_info = if let Some(ref jsummary) = host.jump_chain_summary {
                if jsummary.starts_with("preset:") {
                    let pid = jsummary.trim_start_matches("preset:");
                    if let Ok(Some(tun)) = storage.tunnels().get_by_id(pid).await {
                        let route_sum = tun.route_summary();
                        let hop_len = tun.jump_chain.len() as i32;
                        Some((pid.to_string(), tun.name, route_sum, hop_len))
                    } else {
                        Some((pid.to_string(), String::new(), String::new(), 0))
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let creds = storage.credentials().list_all().await.unwrap_or_default();
            let tunnels = storage.tunnels().list_all().await.unwrap_or_default();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    let hb = w.global::<HostsBridge>();
                    hb.set_test_connection_status("idle".into());
                    hb.set_test_connection_message("".into());

                    // 1. 设置为编辑模式并绑定原主机 ID
                    hb.set_is_edit_mode(true);
                    hb.set_editing_host_id(host.id.clone().into());

                    // 2. 解析回显所属分组
                    let parent_id = host.parent_group_id.clone().unwrap_or_else(|| "root".to_string());
                    hb.set_create_host_default_parent(parent_id.clone().into());
                    hb.set_form_parent_id(parent_id.into());
                    hb.set_form_parent_name(parent_name.into());

                    // 3. 常规信息回显
                    hb.set_form_host_name(host.name.clone().into());
                    hb.set_form_host_address(host.address.clone().into());
                    hb.set_form_host_port_str(host.port.to_string().into());

                    // 4. 凭据与认证模式回显
                    let (cred_id, cred_name, cred_user, cred_type, cred_alg) = cred_info;
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
                    if let Some((pid, name, summary, hop_count)) = preset_tun_info {
                        hb.set_form_preset_jump_id(pid.into());
                        hb.set_form_preset_jump_name(name.into());
                        hb.set_form_preset_jump_summary(summary.into());
                        hb.set_form_preset_jump_hop_count(hop_count);
                    } else {
                        hb.set_form_preset_jump_id("".into());
                        hb.set_form_preset_jump_name("".into());
                        hb.set_form_preset_jump_summary("".into());
                        hb.set_form_preset_jump_hop_count(0);
                    }
                    sync_hosts_bridge_create_host_jump_chain(&w, &[]);

                    // 8. 同步下拉选项数据 (纯内存 0ms)
                    let tree = master_tree.read().unwrap();
                    let expanded_set = selector_expanded.read().unwrap();
                    let group_options = build_group_options(&tree, &expanded_set);
                    sync_hosts_bridge_options(&w, &group_options);

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

                    let jump_options: Vec<JumpHostOptionData> = tree.iter()
                        .filter(|n| !n.is_group)
                        .map(|h| JumpHostOptionData {
                            id: h.id.clone().into(),
                            name: h.name.clone().into(),
                            address: h.address.clone().into(),
                            port: h.port,
                        })
                        .collect();
                    sync_hosts_bridge_jump_hosts(&w, &jump_options);

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

                    hb.set_is_create_host_modal_open(true);
                }
            });
        });
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
            let cat_str = category.to_string().trim().to_lowercase();
            let q_str = query.to_string().trim().to_lowercase();
            let storage = core_state_filter_cred.storage();
            let window_weak = window_weak_filter_cred.clone();

            spawn_async(async move {
                let creds = storage.credentials().list_all().await.unwrap_or_default();
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

                let match_count = filtered.len();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        sync_hosts_bridge_credentials(&w, &filtered);
                    }
                });
                tracing::debug!(target: "smagical_ui::hosts", "新建主机弹窗凭据检索: 分类='{}', 关键字='{}', 匹配条数={}", cat_str, q_str, match_count);
            });
        });
    }

    // -------------------------------------------------------------------------
    // 11.2 代理筛选过滤回调 (按关键字动态检索隧道中心代理通道)
    // -------------------------------------------------------------------------
    {
        let window_weak_filter_proxy = window.as_weak();
        let core_state_filter_proxy = Rc::clone(&ctx.core_state);
        hb.on_filter_create_host_proxies(move |query| {
            let q_str = query.to_string().trim().to_lowercase();
            let storage = core_state_filter_proxy.storage();
            let window_weak = window_weak_filter_proxy.clone();

            spawn_async(async move {
                let tunnels = storage.tunnels().list_all().await.unwrap_or_default();
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

                let match_count = filtered.len();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        sync_hosts_bridge_proxies(&w, &filtered);
                    }
                });
                tracing::debug!(target: "smagical_ui::hosts", "新建主机弹窗代理检索: 关键字='{}', 匹配条数={}", q_str, match_count);
            });
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
                crate::async_util::spawn_async(async move {
                    let target = format!("{}:{}", addr_str, port_num);
                    let start = std::time::Instant::now();
                    let timeout = std::time::Duration::from_secs(5);

                    let result = match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&target)).await {
                        Ok(Ok(_stream)) => Ok(()),
                        Ok(Err(err)) => Err(err),
                        Err(_) => Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            if is_en { "Connection timed out (5s)" } else { "连接超时 (5秒)" },
                        )),
                    };

                    let elapsed = start.elapsed().as_millis();

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(win) = window_weak_bg.upgrade() {
                            let bridge = win.global::<HostsBridge>();
                            match result {
                                Ok(()) => {
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
                });
            }
        });
    }

    // -------------------------------------------------------------------------
    // 13. 提交新建主机配置并同步持久化至存储层
    // -------------------------------------------------------------------------
    {
        let window_weak_submit = window.as_weak();
        let core_state_submit = Rc::clone(&ctx.core_state);
        let master_tree_submit = Arc::clone(&ctx.master_tree);
        let master_cards_submit = Arc::clone(&ctx.master_cards);
        let expanded_submit = Arc::clone(&ctx.expanded_groups);
        let selector_expanded_submit = Arc::clone(&ctx.selector_expanded_groups);
        let search_query_submit = Arc::clone(&ctx.search_query);
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

                let mut new_cred_rec: Option<CredentialRecord> = None;
                if auth_type_str == "credential" {
                    let cid = credential_id.to_string();
                    if !cid.is_empty() {
                        final_cred_id = Some(cid);
                    }
                } else if auth_type_str == "password" {
                    if save_to_credentials && (!pwd_str.is_empty() || !user_str.is_empty()) {
                        let cred_id = format!("cred-{}", uuid::Uuid::new_v4());
                        new_cred_rec = Some(CredentialRecord {
                            id: cred_id.clone(),
                            name: format!("{}-密码凭据", final_name),
                            cred_type: CredentialType::Password,
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
                        });
                        final_cred_id = Some(cred_id);
                    }
                } else if auth_type_str == "key" {
                    if save_to_credentials && !key_str.is_empty() {
                        let cred_id = format!("cred-{}", uuid::Uuid::new_v4());
                        new_cred_rec = Some(CredentialRecord {
                            id: cred_id.clone(),
                            name: format!("{}-私钥凭据", final_name),
                            cred_type: CredentialType::Key,
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
                        });
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

                // 异步写入持久化存储引擎 (0ms UI 阻塞)
                let storage = core_state_submit.storage();
                let events = core_state_submit.events().clone();
                let master_tree = Arc::clone(&master_tree_submit);
                let master_cards = Arc::clone(&master_cards_submit);
                let expanded = Arc::clone(&expanded_submit);
                let selector_expanded = Arc::clone(&selector_expanded_submit);
                let search_query = Arc::clone(&search_query_submit);
                let window_weak = window_weak_submit.clone();
                let notifications = notifications_submit.clone();
                let final_name_clone = final_name.clone();
                let notes_str = notes.to_string();

                spawn_async(async move {
                    let (existing_sort, existing_status, existing_ping) = if is_edit {
                        if let Ok(Some(orig)) = storage.hosts().get_by_id(&target_host_id).await {
                            (orig.sort_order, orig.status, orig.ping_ms)
                        } else {
                            (0, HostStatus::Online, 0)
                        }
                    } else {
                        (0, HostStatus::Online, 0)
                    };

                    if let Some(cred_rec) = new_cred_rec {
                        let _ = storage.credentials().save(&cred_rec).await;
                    }

                    let host_rec = HostRecord {
                        id: target_host_id.clone(),
                        name: final_name_clone.clone(),
                        address: addr_str.clone(),
                        port: port_u16,
                        parent_group_id: final_parent_id.clone(),
                        credential_id: final_cred_id.clone(),
                        status: existing_status,
                        ping_ms: existing_ping,
                        sort_order: existing_sort,
                        notes: notes_str,

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

                    if let Err(e) = storage.hosts().save(&host_rec).await {
                        let _ = slint::invoke_from_event_loop(move || {
                            notifications.error("保存失败", format!("写入存储层受限: {}", e));
                        });
                        return;
                    }

                    // 派发 HostAssetChangedEvent 事件
                    events.dispatch(&HostAssetChangedEvent {
                        host_id: target_host_id.clone(),
                        name: final_name_clone.clone(),
                        address: addr_str,
                        credential_id: final_cred_id,
                        action: if is_edit { "updated".to_string() } else { "created".to_string() },
                    });

                    // 如果属于某个分组，自动将该目标分组及其祖先加入展开集合，保证新建主机立即可见
                    if let Some(ref pid) = final_parent_id {
                        let mut exp = expanded.write().unwrap();
                        let mut curr = pid.clone();
                        let tree = master_tree.read().unwrap();
                        while !curr.is_empty() {
                            exp.insert(curr.clone());
                            if let Some(p) = tree.iter().find(|n| n.id == curr) {
                                curr = p.parent_id.clone();
                            } else {
                                break;
                            }
                        }
                    }

                    // 异步重新组装并渲染 UI
                    sync_ui_hosts_async(
                        storage.as_ref(),
                        &master_tree,
                        &master_cards,
                        &expanded,
                        &selector_expanded,
                        &search_query,
                        window_weak.clone(),
                    ).await;

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = window_weak.upgrade() {
                            let hb = w.global::<HostsBridge>();
                            hb.set_is_create_host_modal_open(false);
                            hb.set_is_edit_mode(false);
                            hb.set_editing_host_id("".into());
                            if is_edit {
                                notifications.success("修改主机成功", format!("主机 [{}] 配置已成功更新", final_name_clone));
                                tracing::info!(target: "smagical_ui::hosts", "成功修改主机配置: {} ({})", final_name_clone, target_host_id);
                            } else {
                                notifications.success("创建主机成功", format!("主机 [{}] 已成功保存至资产列表", final_name_clone));
                                tracing::info!(target: "smagical_ui::hosts", "成功创建新主机记录: {} ({})", final_name_clone, target_host_id);
                            }
                        }
                    });
                });
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
    let master_tree_clone = Arc::clone(&ctx.master_tree);
    let master_cards_clone = Arc::clone(&ctx.master_cards);
    let expanded_clone = Arc::clone(&ctx.expanded_groups);
    let selector_expanded_clone = Arc::clone(&ctx.selector_expanded_groups);
    let search_query_clone = Arc::clone(&ctx.search_query);
    let core_state_clone = Rc::clone(&ctx.core_state);
    let notifications_clone = ctx.notifications.clone();
    hb.on_clone_host(move |host_id| {
        let hid = host_id.to_string();
        let storage = core_state_clone.storage();
        let events = core_state_clone.events().clone();
        let master_tree = Arc::clone(&master_tree_clone);
        let master_cards = Arc::clone(&master_cards_clone);
        let expanded = Arc::clone(&expanded_clone);
        let selector_expanded = Arc::clone(&selector_expanded_clone);
        let search_query = Arc::clone(&search_query_clone);
        let window_weak_bg = window_weak.clone();
        let notifications = notifications_clone.clone();

        spawn_async(async move {
            if let Ok(Some(orig)) = storage.hosts().get_by_id(&hid).await {
                let new_id = format!("host-{}", uuid::Uuid::new_v4().simple());
                let cloned_name = format!("{} (副本)", orig.name);
                let mut cloned_rec = orig.clone();
                cloned_rec.id = new_id.clone();
                cloned_rec.name = cloned_name.clone();
                cloned_rec.sort_order += 1;
                if let Err(e) = storage.hosts().save(&cloned_rec).await {
                    let _ = slint::invoke_from_event_loop(move || {
                        notifications.error("克隆失败", format!("保存至数据层受限: {}", e));
                    });
                    return;
                }

                events.dispatch(&HostAssetChangedEvent {
                    host_id: new_id.clone(),
                    name: cloned_name.clone(),
                    address: cloned_rec.address.clone(),
                    credential_id: None,
                    action: "cloned".to_string(),
                });

                // 异步刷新树与卡片 (0ms UI 阻塞)
                sync_ui_hosts_async(
                    storage.as_ref(),
                    &master_tree,
                    &master_cards,
                    &expanded,
                    &selector_expanded,
                    &search_query,
                    window_weak_bg,
                ).await;

                let _ = slint::invoke_from_event_loop(move || {
                    notifications.success("克隆主机成功", format!("主机 [{}] 已成功创建副本", cloned_name));
                });
            }
        });
    });

    // -------------------------------------------------------------------------
    // 18. 右键菜单 - 删除主机或分组节点
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let master_tree_del = Arc::clone(&ctx.master_tree);
    let master_cards_del = Arc::clone(&ctx.master_cards);
    let expanded_del = Arc::clone(&ctx.expanded_groups);
    let selector_expanded_del = Arc::clone(&ctx.selector_expanded_groups);
    let search_query_del = Arc::clone(&ctx.search_query);
    let core_state_del = Rc::clone(&ctx.core_state);
    let notifications_del = ctx.notifications.clone();
    hb.on_delete_node(move |target_id, is_group| {
        let tid = target_id.to_string();
        let storage = core_state_del.storage();
        let events = core_state_del.events().clone();
        let master_tree = Arc::clone(&master_tree_del);
        let master_cards = Arc::clone(&master_cards_del);
        let expanded = Arc::clone(&expanded_del);
        let selector_expanded = Arc::clone(&selector_expanded_del);
        let search_query = Arc::clone(&search_query_del);
        let window_weak_bg = window_weak.clone();
        let notifications = notifications_del.clone();

        spawn_async(async move {
            if is_group {
                // 删除分组前，将其直属主机移入根目录，避免数据孤儿
                if let Ok(hosts) = storage.hosts().list_all().await {
                    for mut h in hosts {
                        if h.parent_group_id.as_deref() == Some(&tid) {
                            h.parent_group_id = None;
                            let _ = storage.hosts().save(&h).await;
                        }
                    }
                }
                let _ = storage.groups().delete(&tid).await;
                expanded.write().unwrap().remove(&tid);
                selector_expanded.write().unwrap().remove(&tid);

                let _ = slint::invoke_from_event_loop({
                    let n = notifications.clone();
                    move || {
                        n.success("删除成功", "分组已删除，内部主机已安全移至根目录");
                    }
                });
            } else {
                let _ = storage.hosts().delete(&tid).await;
                let _ = slint::invoke_from_event_loop({
                    let n = notifications.clone();
                    move || {
                        n.success("删除成功", "目标主机资产已从列表中移除");
                    }
                });
            }

            events.dispatch(&HostAssetChangedEvent {
                host_id: tid.clone(),
                name: "".to_string(),
                address: "".to_string(),
                credential_id: None,
                action: "deleted".to_string(),
            });

            // 异步刷新树与卡片 (0ms UI 阻塞)
            sync_ui_hosts_async(
                storage.as_ref(),
                &master_tree,
                &master_cards,
                &expanded,
                &selector_expanded,
                &search_query,
                window_weak_bg,
            ).await;
        });
    });
}
