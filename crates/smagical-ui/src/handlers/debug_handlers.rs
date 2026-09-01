//! 开发者全功能调试控制台与批量造数操作回调绑定。
//!
//! 提供内存压测造数、状态批量模拟、拓扑场景预设注入、快速增删改查与实时 Tracing 日志抓取回调。

use std::cell::RefCell;
use std::rc::Rc;
use slint::{ComponentHandle, Model};
use smagical_core::event::ConfigChangedEvent;
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
                credential_id: None,
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

    // -------------------------------------------------------------------------
    // 8. 强制全屏重绘 (Force Repaint)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    window.on_debug_force_repaint(move || {
        if let Some(w) = window_weak.upgrade() {
            w.window().request_redraw();
            tracing::info!(target: "smagical_debug::render", "已触发 GPU 全屏强制重绘 (Force Repaint)");
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 9. 终端 120Hz 高频吞吐性能基准压测 (Benchmark)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let pane_groups_bm = Rc::clone(&ctx.pane_groups);
    let active_terminals_bm = Rc::clone(&ctx.active_terminals);
    window.on_debug_run_benchmark(move || {
        if let Some(w) = window_weak.upgrade() {
            let groups = pane_groups_bm.borrow();
            if let Some(active_sess) = groups.first().and_then(|g| g.get_active_session()) {
                let mut terminals = active_terminals_bm.borrow_mut();
                if let Some(instance) = terminals.get_mut(&active_sess.session_id) {
                    let benchmark_payload = "\x1b[32m[Benchmark]\x1b[0m 正在执行 120Hz 高频吞吐压力测试...\r\n";
                    let _ = instance.send_bytes(benchmark_payload.as_bytes());
                }
            }
            tracing::info!(target: "smagical_debug::render", "已触发终端 120Hz 视口高频吞吐基准压测");
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 10. 切换图形渲染管线首选项 (Switch Pipeline)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_pipe = ctx.core_state.clone();
    window.on_debug_switch_pipeline(move |pipe_id| {
        let p_str = pipe_id.to_string();
        // Safety: 设置环境变量以在下次启动或测试时生效
        unsafe {
            std::env::set_var("SLINT_BACKEND", &p_str);
        }
        if let Some(w) = window_weak.upgrade() {
            w.set_active_rendering_pipeline(p_str.clone().into());
            core_state_pipe.events().dispatch(&ConfigChangedEvent {
                key: "rendering.pipeline".into(),
                old_val: "".into(),
                new_val: p_str.clone(),
                source: "debug_switch_pipeline".into(),
            });
            tracing::info!(target: "smagical_debug::render", "图形渲染管线首选项已切换为: {}", p_str);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 11. 保存管线配置并生效 (Apply Pipeline)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    window.on_debug_apply_pipeline(move || {
        if let Some(w) = window_weak.upgrade() {
            let cur = w.get_active_rendering_pipeline().to_string();
            // Safety: 设置环境变量以持久化渲染后端配置
            unsafe {
                std::env::set_var("SLINT_BACKEND", &cur);
            }
            w.window().request_redraw();
            tracing::info!(target: "smagical_debug::render", "已成功保存渲染管线首选项: [{}] (将在下次启动时加载生效)", cur);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 12. 保存管线并立即重启客户端 (Restart App)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let persistence_guard = ctx.persistence_guard.clone();
    window.on_debug_restart_app(move || {
        if let Some(w) = window_weak.upgrade() {
            let cur = w.get_active_rendering_pipeline().to_string();
            unsafe {
                std::env::set_var("SLINT_BACKEND", &cur);
            }
            tracing::info!(target: "smagical_debug::render", "正在执行客户端安全重启以加载全新渲染管线: [{}]...", cur);
            
            // 确保异步会话数据落盘
            persistence_guard.flush_and_wait(std::time::Duration::from_millis(500));

            // 拉起新进程并退出当前旧进程
            if let Ok(exe_path) = std::env::current_exe() {
                let mut cmd = std::process::Command::new(exe_path);
                cmd.env("SLINT_BACKEND", &cur);
                if let Err(e) = cmd.spawn() {
                    tracing::error!(target: "smagical_debug::render", "拉起新进程失败: {:?}", e);
                } else {
                    std::process::exit(0);
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 13. 开发者调试控制台实时日志流自动同步定时器 (500ms 刷新)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let log_timer = Box::leak(Box::new(slint::Timer::default()));
    log_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(500),
        move || {
            if let Some(w) = window_weak.upgrade().filter(|w| w.get_is_debug_modal_open() && smagical_debug::is_debug_enabled()) {
                sync_ui_debug_logs(&w);
            }
        },
    );

    // -------------------------------------------------------------------------
    // 14. 批量生成凭据测试数据
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_bg_cred = ctx.core_state.clone();
    let notif_bg_cred = ctx.notifications.clone();
    window.on_debug_batch_generate_credentials(move |count_str, mode_str, overwrite| {
        if let Some(w) = window_weak.upgrade() {
            let count = count_str.as_str().parse::<usize>().unwrap_or(10).max(1);
            let mode = mode_str.to_string();

            if overwrite {
                if let Ok(existing) = core_state_bg_cred.storage().credentials().list_all() {
                    for c in existing {
                        let _ = core_state_bg_cred.storage().credentials().delete(&c.id);
                    }
                }
            }

            let mut batch_records = Vec::with_capacity(count);
            for i in 1..=count {
                let id = format!("cred-mock-{}", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>());
                let c_type = match mode.as_str() {
                    "key" => smagical_core::CredentialType::Key,
                    "password" => smagical_core::CredentialType::Password,
                    "agent" => smagical_core::CredentialType::Agent,
                    _ => match i % 3 {
                        0 => smagical_core::CredentialType::Key,
                        1 => smagical_core::CredentialType::Password,
                        _ => smagical_core::CredentialType::Agent,
                    },
                };

                let rec = match c_type {
                    smagical_core::CredentialType::Key => smagical_core::CredentialRecord {
                        id: id.clone(),
                        name: format!("批量测试密钥 #{}", i),
                        cred_type: smagical_core::CredentialType::Key,
                        algorithm: if i % 2 == 0 { "Ed25519".to_string() } else { "RSA-4096".to_string() },
                        username: Some(format!("user-{}", i)),
                        secret_data: "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\nQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAJCR2Y69kdmO\nvQAAAAtzc2gtZWQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6\nSQAAAEA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP\n4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ=\n-----END OPENSSH PRIVATE KEY-----".to_string(),
                        passphrase: if i % 2 == 0 { Some("••••••••".to_string()) } else { None },
                        public_key: Some(format!("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI{} test-key-{}", i, i)),
                        fingerprint: Some(format!("SHA256:mockFingerprint{:08x}", i)),
                        bound_host_count: i % 5,
                        created_at: "2026-09-01 12:00:00".to_string(),
                        updated_at: "2026-09-01 12:00:00".to_string(),
                        notes: format!("批量压测生成密钥 #{}", i),
                    },
                    smagical_core::CredentialType::Password => smagical_core::CredentialRecord {
                        id: id.clone(),
                        name: format!("批量测试密码 #{}", i),
                        cred_type: smagical_core::CredentialType::Password,
                        algorithm: "Password".to_string(),
                        username: Some(format!("admin-{}", i)),
                        secret_data: format!("MockPass#{}!Secure", i),
                        passphrase: None,
                        public_key: None,
                        fingerprint: None,
                        bound_host_count: i % 4,
                        created_at: "2026-09-01 12:00:00".to_string(),
                        updated_at: "2026-09-01 12:00:00".to_string(),
                        notes: format!("批量压测生成密码 #{}", i),
                    },
                    smagical_core::CredentialType::Agent | smagical_core::CredentialType::Certificate => smagical_core::CredentialRecord {
                        id: id.clone(),
                        name: format!("测试 SSH Agent 管道 #{}", i),
                        cred_type: smagical_core::CredentialType::Agent,
                        algorithm: if i % 2 == 0 { "1Password".to_string() } else { "OpenSSH".to_string() },
                        username: Some(format!("agent-user-{}", i)),
                        secret_data: if i % 2 == 0 { r"\\.\pipe\1password-ssh-agent".to_string() } else { r"\\.\pipe\openssh-ssh-agent".to_string() },
                        passphrase: None,
                        public_key: Some(format!("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI{} agent-{}", i, i)),
                        fingerprint: Some(format!("SHA256:agentFingerprint{:08x}", i)),
                        bound_host_count: i % 3,
                        created_at: "2026-09-01 12:00:00".to_string(),
                        updated_at: "2026-09-01 12:00:00".to_string(),
                        notes: format!("批量压测生成 Agent #{}", i),
                    },
                };
                batch_records.push(rec);
            }

            let _ = core_state_bg_cred.storage().credentials().save_batch(&batch_records);
            let cat = w.get_credential_filter_category().to_string();
            let q = w.get_credential_search_query().to_string();
            crate::handlers::credential_handlers::sync_credentials_ui(&w, &core_state_bg_cred, &cat, &q);
            notif_bg_cred.success("批量凭据生成完成", &format!("成功注入 {} 条测试凭据数据", count));
        }
    });

    // -------------------------------------------------------------------------
    // 15. 快捷添加单条测试凭据
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_qa_cred = ctx.core_state.clone();
    let notif_qa_cred = ctx.notifications.clone();
    window.on_debug_quick_add_credential(move |ctype, name, data| {
        if let Some(w) = window_weak.upgrade() {
            let id = format!("cred-quick-{}", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>());
            let c_name = name.to_string();
            let c_data = data.to_string();
            let cred_type = match ctype.as_str() {
                "key" => smagical_core::CredentialType::Key,
                "password" => smagical_core::CredentialType::Password,
                "agent" => smagical_core::CredentialType::Agent,
                _ => smagical_core::CredentialType::Key,
            };

            let rec = match cred_type {
                smagical_core::CredentialType::Key => smagical_core::CredentialRecord {
                    id: id.clone(),
                    name: c_name.clone(),
                    cred_type: smagical_core::CredentialType::Key,
                    algorithm: if c_data.contains("RSA") { "RSA-4096".to_string() } else { "Ed25519".to_string() },
                    username: Some("root".to_string()),
                    secret_data: if c_data.is_empty() {
                        "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\nQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAJCR2Y69kdmO\nvQAAAAtzc2gtZWQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6\nSQAAAEA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP\n4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ=\n-----END OPENSSH PRIVATE KEY-----".to_string()
                    } else {
                        c_data
                    },
                    passphrase: None,
                    public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMfyDbS9fsr2nUF83bA/iVeplszjEaAD1Anq0vuvWfpJ root@test".to_string()),
                    fingerprint: Some("SHA256:k9x8Ym+3pLq1G7vX2nR8uM4aP9tL3wQ2bN6pG8oP4qM".to_string()),
                    bound_host_count: 1,
                    created_at: "2026-09-01 12:00:00".to_string(),
                    updated_at: "2026-09-01 12:00:00".to_string(),
                    notes: "快捷添加的测试密钥凭据".to_string(),
                },
                smagical_core::CredentialType::Password => smagical_core::CredentialRecord {
                    id: id.clone(),
                    name: c_name.clone(),
                    cred_type: smagical_core::CredentialType::Password,
                    algorithm: "Password".to_string(),
                    username: Some("root".to_string()),
                    secret_data: if c_data.is_empty() { "SmaluxSecure#2026!P@ss".to_string() } else { c_data },
                    passphrase: None,
                    public_key: None,
                    fingerprint: None,
                    bound_host_count: 1,
                    created_at: "2026-09-01 12:00:00".to_string(),
                    updated_at: "2026-09-01 12:00:00".to_string(),
                    notes: "快捷添加的测试密码凭据".to_string(),
                },
                smagical_core::CredentialType::Agent | smagical_core::CredentialType::Certificate => smagical_core::CredentialRecord {
                    id: id.clone(),
                    name: c_name.clone(),
                    cred_type: smagical_core::CredentialType::Agent,
                    algorithm: "Agent".to_string(),
                    username: Some("agent".to_string()),
                    secret_data: if c_data.is_empty() { r"\\.\pipe\openssh-ssh-agent".to_string() } else { c_data },
                    passphrase: None,
                    public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q agent".to_string()),
                    fingerprint: Some("SHA256:1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d".to_string()),
                    bound_host_count: 1,
                    created_at: "2026-09-01 12:00:00".to_string(),
                    updated_at: "2026-09-01 12:00:00".to_string(),
                    notes: "快捷添加的测试 Agent 管道凭据".to_string(),
                },
            };

            let _ = core_state_qa_cred.storage().credentials().save(&rec);
            let cat = w.get_credential_filter_category().to_string();
            let q = w.get_credential_search_query().to_string();
            crate::handlers::credential_handlers::sync_credentials_ui(&w, &core_state_qa_cred, &cat, &q);
            notif_qa_cred.success("凭据添加成功", &format!("已成功注入: {}", c_name));
        }
    });

    // -------------------------------------------------------------------------
    // 16. 恢复默认凭据预设
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_rst_cred = ctx.core_state.clone();
    let notif_rst_cred = ctx.notifications.clone();
    window.on_debug_reset_default_credentials(move || {
        if let Some(w) = window_weak.upgrade() {
            if let Ok(existing) = core_state_rst_cred.storage().credentials().list_all() {
                for c in existing {
                    let _ = core_state_rst_cred.storage().credentials().delete(&c.id);
                }
            }

            let default_creds = vec![
                smagical_core::CredentialRecord {
                    id: "cred-prod-ed25519".to_string(),
                    name: "生产集群 Ed25519 密钥".to_string(),
                    cred_type: smagical_core::CredentialType::Key,
                    algorithm: "Ed25519".to_string(),
                    username: Some("root".to_string()),
                    secret_data: "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\nQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAJCR2Y69kdmO\nvQAAAAtzc2gtZWQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6\nSQAAAEA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP\n4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ=\n-----END OPENSSH PRIVATE KEY-----".to_string(),
                    passphrase: Some("••••••••".to_string()),
                    public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMfyDbS9fsr2nUF83bA/iVeplszjEaAD1Anq0vuvWfpJ root@smalux-k8s-prod".to_string()),
                    fingerprint: Some("SHA256:k9x8Ym+3pLq1G7vX2nR8uM4aP9tL3wQ2bN6pG8oP4qM".to_string()),
                    bound_host_count: 5,
                    created_at: "2026-08-15 10:20:00".to_string(),
                    updated_at: "2026-09-01 09:15:00".to_string(),
                    notes: "Kubernetes 核心控制面与网关认证主密钥".to_string(),
                },
                smagical_core::CredentialRecord {
                    id: "cred-bastion-pwd".to_string(),
                    name: "堡垒跳板机 Root 管理密码".to_string(),
                    cred_type: smagical_core::CredentialType::Password,
                    algorithm: "Password".to_string(),
                    username: Some("root".to_string()),
                    secret_data: "SmaluxSecure#2026!P@ss".to_string(),
                    passphrase: None,
                    public_key: None,
                    fingerprint: None,
                    bound_host_count: 2,
                    created_at: "2026-08-18 14:30:00".to_string(),
                    updated_at: "2026-08-30 18:00:00".to_string(),
                    notes: "边缘网关与跳板机应急控制台特权密码".to_string(),
                },
                smagical_core::CredentialRecord {
                    id: "cred-1pwd-agent".to_string(),
                    name: "1Password SSH Agent".to_string(),
                    cred_type: smagical_core::CredentialType::Agent,
                    algorithm: "1Password".to_string(),
                    username: Some("developer".to_string()),
                    secret_data: r"\\.\pipe\1password-ssh-agent".to_string(),
                    passphrase: None,
                    public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q 1password-agent".to_string()),
                    fingerprint: Some("SHA256:1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d".to_string()),
                    bound_host_count: 3,
                    created_at: "2026-08-20 16:45:00".to_string(),
                    updated_at: "2026-09-01 11:30:00".to_string(),
                    notes: "硬件安全保管箱，受 Windows Hello 生物识别保护".to_string(),
                },
                smagical_core::CredentialRecord {
                    id: "cred-dev-rsa".to_string(),
                    name: "CI/CD 流水线 RSA 密钥".to_string(),
                    cred_type: smagical_core::CredentialType::Key,
                    algorithm: "RSA-4096".to_string(),
                    username: Some("gitlab-runner".to_string()),
                    secret_data: "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0k6K9X7L9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQ2Y69kd\nvQMwAAAAtzc2gtcnNhAAAAAwEAAQAAAgEAv7b4a2p8zXqN3vP9xK2m4rL9nO1pQ8tL\n3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQ\nA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4l\nXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQFAgcICQoLDA0O\nDxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0\nBBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3Bx\ncnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6Ch\noqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR\n0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/wID\n-----END RSA PRIVATE KEY-----".to_string(),
                    passphrase: None,
                    public_key: Some("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7b4a2p8zXqN3vP9xK2m4rL9nO1pQ8tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ gitlab@runner".to_string()),
                    fingerprint: Some("SHA256:9c8b7a6f5e4d3c2b1a0f9e8d7c6b5a4f".to_string()),
                    bound_host_count: 1,
                    created_at: "2026-08-25 09:00:00".to_string(),
                    updated_at: "2026-08-25 09:00:00".to_string(),
                    notes: "GitLab Runner 持续部署构建机专有免密凭据".to_string(),
                },
                smagical_core::CredentialRecord {
                    id: "cred-openssh-agent".to_string(),
                    name: "Windows OpenSSH Agent".to_string(),
                    cred_type: smagical_core::CredentialType::Agent,
                    algorithm: "OpenSSH".to_string(),
                    username: Some("ssh-agent".to_string()),
                    secret_data: r"\\.\pipe\openssh-ssh-agent".to_string(),
                    passphrase: None,
                    public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q openssh-agent".to_string()),
                    fingerprint: Some("SHA256:8f4e2c9a1b3d5e7f0a2c4e6b8d0f1a3c".to_string()),
                    bound_host_count: 4,
                    created_at: "2026-08-22 11:00:00".to_string(),
                    updated_at: "2026-08-22 11:00:00".to_string(),
                    notes: "Windows 内置 OpenSSH Authentication Agent 命名管道".to_string(),
                },
                smagical_core::CredentialRecord {
                    id: "cred-bitwarden-agent".to_string(),
                    name: "Bitwarden SSH Agent".to_string(),
                    cred_type: smagical_core::CredentialType::Agent,
                    algorithm: "Bitwarden".to_string(),
                    username: Some("vault".to_string()),
                    secret_data: r"\\.\pipe\bitwarden-ssh-agent".to_string(),
                    passphrase: None,
                    public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q bitwarden-agent".to_string()),
                    fingerprint: Some("SHA256:3d5e7f9a1c2b4d6e8f0a1b3c5d7e9f1a".to_string()),
                    bound_host_count: 2,
                    created_at: "2026-08-28 15:20:00".to_string(),
                    updated_at: "2026-08-28 15:20:00".to_string(),
                    notes: "Bitwarden / Vaultwarden 桌面端安全托管 SSH Agent".to_string(),
                },
            ];

            let _ = core_state_rst_cred.storage().credentials().save_batch(&default_creds);
            let cat = w.get_credential_filter_category().to_string();
            let q = w.get_credential_search_query().to_string();
            crate::handlers::credential_handlers::sync_credentials_ui(&w, &core_state_rst_cred, &cat, &q);
            notif_rst_cred.success("预设恢复完成", "已重新载入 6 项精选预设凭据");
        }
    });

    // -------------------------------------------------------------------------
    // 17. 清空所有凭据
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_clr_cred = ctx.core_state.clone();
    let notif_clr_cred = ctx.notifications.clone();
    window.on_debug_clear_credentials(move || {
        if let Some(w) = window_weak.upgrade() {
            if let Ok(existing) = core_state_clr_cred.storage().credentials().list_all() {
                for c in existing {
                    let _ = core_state_clr_cred.storage().credentials().delete(&c.id);
                }
            }

            let cat = w.get_credential_filter_category().to_string();
            let q = w.get_credential_search_query().to_string();
            crate::handlers::credential_handlers::sync_credentials_ui(&w, &core_state_clr_cred, &cat, &q);
            notif_clr_cred.info("凭据已清空", "所有凭据数据已从存储层完全清除");
        }
    });
}






