//! 开发者全功能调试控制台与批量造数操作回调绑定。
//!
//! 提供内存压测造数、状态批量模拟、拓扑场景预设注入、快速增删改查与实时 Tracing 日志抓取回调。

use std::cell::RefCell;
use std::rc::Rc;
use slint::{ComponentHandle, Model};
use smagical_debug::{
    generate_batch_hosts, get_preset_by_id, BatchGenerateConfig,
};

use crate::debug_ui::sync_ui_debug_logs;
use crate::generated::{AppWindow, HostItemData};
use crate::handlers::AppContext;
use crate::tree_model::{
    build_group_options, build_search_tree_nodes, build_visible_tree_nodes,
    calculate_max_tree_width, ensure_raw_group_hierarchy, RawTreeNode,
};

/// 注册所有开发者调试控制台相关回调。
///
/// 绑定批量生成、状态模拟、端口批量变更、预设注入、快速增改、数据重置与测试日志等所有 F12 调试面板事件。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用
/// - `ctx`: 全局应用共享上下文对象引用
pub(crate) fn register_debug_handlers(window: &AppWindow, ctx: &AppContext) {
    // -------------------------------------------------------------------------
    // 0.1 批量生成主机资产
    // -------------------------------------------------------------------------
    // 支持按前缀、数量、起始 IP、目标分组与状态模式快速生成大规模主机资产，支持“追加 (Append)”或“覆盖 (Overwrite)”。
    let window_weak = window.as_weak();
    let master_tree_bg = Rc::clone(&ctx.master_tree);
    let expanded_bg = Rc::clone(&ctx.expanded_groups);
    let selector_bg = Rc::clone(&ctx.selector_expanded_groups);
    let search_bg = Rc::clone(&ctx.search_query);
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

            tracing::info!(target: "smagical_debug::batch", "批量生成主机资产完成 (共 {} 台, 挂载分组: {})", cnt, grp_str);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 0.2 批量更新主机状态
    // -------------------------------------------------------------------------
    // 支持将所有主机一键切换为 "all_online" (全在线), "all_offline" (全离线), "all_warning" (全告警) 或 "mixed" (混合)。
    let window_weak = window.as_weak();
    let master_tree_bs = Rc::clone(&ctx.master_tree);
    let expanded_bs = Rc::clone(&ctx.expanded_groups);
    let search_bs = Rc::clone(&ctx.search_query);
    let core_state_bs = Rc::clone(&ctx.core_state);
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

            // 同步批量状态更新至存储层
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

    // -------------------------------------------------------------------------
    // 0.3 批量更新 SSH 端口
    // -------------------------------------------------------------------------
    // 一键修改全量主机的 SSH 连接端口 (例如由 22 切换为 2222)。
    let window_weak = window.as_weak();
    let master_tree_bp = Rc::clone(&ctx.master_tree);
    let expanded_bp = Rc::clone(&ctx.expanded_groups);
    let search_bp = Rc::clone(&ctx.search_query);
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

            tracing::info!(target: "smagical_debug::batch", "批量修改全量主机 SSH 端口为: {}", new_port);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 1. 注入调试场景预设
    // -------------------------------------------------------------------------
    // 注入内置标准测试数据集 (如: "deep_nested" 6级深度树, "massive_100" 百台机器, "minimal" 精简集合)。
    let window_weak = window.as_weak();
    let master_tree_inj = Rc::clone(&ctx.master_tree);
    let expanded_inj = Rc::clone(&ctx.expanded_groups);
    let selector_inj = Rc::clone(&ctx.selector_expanded_groups);
    let search_inj = Rc::clone(&ctx.search_query);
    window.on_debug_inject_preset(move |preset_id| {

        if let Some(w) = window_weak.upgrade() {
            let p_id = preset_id.as_str();
            let (new_tree_raw, new_cards_raw) = get_preset_by_id(p_id);
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

            *master_tree_inj.borrow_mut() = new_tree.clone();
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(new_cards))));

            // 预设注入后展开所有顶级及二级分组
            let mut exp = expanded_inj.borrow_mut();
            let mut sel = selector_inj.borrow_mut();
            exp.clear();
            sel.clear();
            for n in &new_tree {
                if n.is_group {
                    exp.insert(n.id.clone());
                    sel.insert(n.id.clone());
                }
            }

            let opts = build_group_options(&new_tree, &sel);
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let q = search_inj.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&new_tree, &exp)
            } else {
                build_search_tree_nodes(&new_tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::preset", "已成功注入调试场景预设: [{}] (共 {} 个节点)", p_id, new_tree.len());
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 2. 快速新增主机
    // -------------------------------------------------------------------------
    // 支持直接指定斜杠嵌套路径（如: "集群/k8s-master"），自动创建父级分组并同步保存至存储层。
    let window_weak = window.as_weak();
    let master_tree_qh = Rc::clone(&ctx.master_tree);
    let expanded_qh = Rc::clone(&ctx.expanded_groups);
    let selector_qh = Rc::clone(&ctx.selector_expanded_groups);
    let search_qh = Rc::clone(&ctx.search_query);
    let core_state_qh = Rc::clone(&ctx.core_state);
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

            // 同步新增主机至存储层，避免与 storage 双真相来源分叉
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

    // -------------------------------------------------------------------------
    // 3. 快速新增分组
    // -------------------------------------------------------------------------
    // 支持按多级路径直接创建嵌套分组（如: "华东/上海/开发环境"）。
    let window_weak = window.as_weak();
    let master_tree_qg = Rc::clone(&ctx.master_tree);
    let expanded_qg = Rc::clone(&ctx.expanded_groups);
    let selector_qg = Rc::clone(&ctx.selector_expanded_groups);
    let search_qg = Rc::clone(&ctx.search_query);
    window.on_debug_quick_add_group(move |name, _parent| {
        if let Some(w) = window_weak.upgrade() {
            let g_name = name.trim().to_string();
            if g_name.is_empty() { return; }

            let mut tree = master_tree_qg.borrow_mut();
            let (_leaf_id, _leaf_lvl, _leaf_name) = ensure_raw_group_hierarchy(&mut tree, &g_name);

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

            tracing::info!(target: "smagical_debug::data", "快速创建嵌套分组: {}", g_name);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 4. 清空全量数据
    // -------------------------------------------------------------------------
    // 一键清空内存树形缓存、列表模型并彻底清空存储层中所有主机与分组记录。
    let window_weak = window.as_weak();
    let master_tree_clr = Rc::clone(&ctx.master_tree);
    let core_state_clr = Rc::clone(&ctx.core_state);
    window.on_debug_clear_data(move || {
        if let Some(w) = window_weak.upgrade() {
            master_tree_clr.borrow_mut().clear();
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::<crate::generated::HostTreeNode>::new()))));
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::<crate::generated::HostItemData>::new()))));
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::<crate::generated::GroupOptionData>::new()))));
            w.set_tree_content_width(240.0_f32);
            // 同步清空存储层
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

    // -------------------------------------------------------------------------
    // 5. 恢复默认精简数据
    // -------------------------------------------------------------------------
    // 将内存数据模型与界面重置恢复至系统内置默认数据集 (Minimal 预设)。
    let window_weak = window.as_weak();
    let master_tree_rst = Rc::clone(&ctx.master_tree);
    let expanded_rst = Rc::clone(&ctx.expanded_groups);
    let selector_rst = Rc::clone(&ctx.selector_expanded_groups);
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
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(def_cards))));

            let mut exp = expanded_rst.borrow_mut();
            let mut sel = selector_rst.borrow_mut();
            exp.clear();
            sel.clear();
            for n in &def_tree {
                if n.is_group {
                    exp.insert(n.id.clone());
                    sel.insert(n.id.clone());
                }
            }

            let opts = build_group_options(&def_tree, &sel);
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let next_nodes = build_visible_tree_nodes(&def_tree, &exp);
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::data", "已重置恢复至默认精简数据集 (Minimal)");
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 6. 清空诊断日志
    // -------------------------------------------------------------------------
    // 清空全局内存 Tracing 日志缓冲区与 Slint 前端日志列表。
    let window_weak = window.as_weak();
    window.on_debug_clear_logs(move || {
        if let Some(w) = window_weak.upgrade() {
            if let Ok(mut buf) = smagical_debug::get_global_log_buffer().lock() {
                buf.clear();
            }
            w.set_debug_logs(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::new()))));
            tracing::info!(target: "smagical_debug::log", "调试日志已清空");
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 7. 发送测试诊断日志
    // -------------------------------------------------------------------------
    // 一键向 Tracing 日志流注入 INFO / WARN / ERROR 三种级别的模拟诊断日志。
    let window_weak = window.as_weak();
    window.on_debug_emit_test_log(move |_level| {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_debug::mock", "这是一条 INFO 测试诊断日志");
            tracing::warn!(target: "smagical_debug::mock", "这是一条 WARN 警告诊断日志");
            tracing::error!(target: "smagical_debug::mock", "这是一条 ERROR 错误诊断日志");
            sync_ui_debug_logs(&w);
        }
    });
}

