//! 网络隧道、跳板机与代理管理中心业务回调处理器。
//!
//! 负责规则多维过滤、启停控制、实时拓扑与指标监控、配置保存与删除、原生 OpenSSH 命令生成与复制。

use slint::{ComponentHandle, Model, ModelRc, VecModel};
use smagical_core::domain::tunnel::{TunnelRecord, TunnelType};
use smagical_core::event::{
    TunnelBeforeDeleteEvent, TunnelBeforeSaveEvent, TunnelDeletedEvent, TunnelSavedEvent,
    TunnelStateChangedEvent,
};

use crate::generated::{AppWindow, JumpHopData, TunnelItemData};
use crate::handlers::AppContext;

fn update_jump_command_preview(w: &AppWindow, hops: &[JumpHopData]) {
    let active_hops: Vec<String> = hops.iter()
        .filter(|h| h.enabled)
        .map(|h| {
            let addr = if h.host_address.is_empty() { h.host_name.to_string() } else { h.host_address.to_string() };
            if h.host_port == 22 || h.host_port == 0 {
                addr
            } else {
                format!("{}:{}", addr, h.host_port)
            }
        })
        .collect();
    let cmd = if !active_hops.is_empty() {
        format!("ssh -J {} target-user@target-host", active_hops.join(","))
    } else {
        "-J <尚未选择启用跳板节点>".to_string()
    };
    w.set_tunnel_form_ssh_command(cmd.into());
}

/// 同步并刷新 UI 网络隧道与代理规则列表
pub(crate) fn sync_ui_tunnels(window: &AppWindow, ctx: &AppContext) {
    let all_tunnels = ctx.core_state.storage().tunnels().list_all().unwrap_or_default();

    let cat = ctx.tunnel_filter_category.borrow().clone();
    let query = ctx.tunnel_search_query.borrow().trim().to_lowercase();

    let filtered: Vec<TunnelItemData> = all_tunnels
        .into_iter()
        .filter(|t| {
            // 1. 分类过滤
            let match_cat = match cat.as_str() {
                "all" => true,
                "forward" => matches!(t.tunnel_type, TunnelType::Local | TunnelType::Remote | TunnelType::Dynamic | TunnelType::ReverseDynamic),
                "jump" => matches!(t.tunnel_type, TunnelType::JumpHost),
                "proxy" => matches!(t.tunnel_type, TunnelType::ProxyServer),
                _ => true,
            };
            if !match_cat {
                return false;
            }

            // 2. 关键词模糊搜索
            if query.is_empty() {
                true
            } else {
                t.name.to_lowercase().contains(&query)
                    || t.remote_host.to_lowercase().contains(&query)
                    || t.local_port.to_string().contains(&query)
                    || t.remote_port.to_string().contains(&query)
                    || t.ssh_host_name.to_lowercase().contains(&query)
                    || t.notes.to_lowercase().contains(&query)
            }
        })
        .map(|t| {
            let (traffic_in, traffic_out) = t.formatted_traffic();
            let host_addr = t.ssh_host_name.clone();
            let ssh_cmd = t.generate_ssh_command(&host_addr, "root");
            let route_summary = t.route_summary();

            TunnelItemData {
                id: t.id.into(),
                name: t.name.into(),
                tunnel_type: t.tunnel_type.as_str().into(),
                type_badge: t.tunnel_type.display_badge().into(),
                ssh_host_id: t.ssh_host_id.unwrap_or_default().into(),
                ssh_host_name: t.ssh_host_name.into(),
                local_bind: t.local_bind.into(),
                local_port: t.local_port as i32,
                remote_host: t.remote_host.into(),
                remote_port: t.remote_port as i32,
                route_summary: route_summary.into(),
                is_running: t.is_running,
                auto_start: t.auto_start,
                auto_reconnect: t.auto_reconnect,
                remote_dns: t.remote_dns,
                compression: t.compression,
                active_connections: t.active_connections as i32,
                traffic_in: traffic_in.into(),
                traffic_out: traffic_out.into(),
                notes: t.notes.into(),
                updated_at: t.updated_at.into(),
                ssh_command: ssh_cmd.into(),
            }
        })
        .collect();

    window.set_tunnel_filter_category(cat.clone().into());
    window.set_tunnel_search_query(query.clone().into());
    window.set_tunnels(ModelRc::new(VecModel::from(filtered.clone())));

    // 如果当前选中的规则不在当前过滤结果列表中，自动选中第一条有效规则并加载其表单详情
    let current_id = window.get_active_tunnel_id().to_string();
    let contains_current = filtered.iter().any(|t| t.id == current_id);
    if !contains_current {
        if let Some(first) = filtered.first() {
            let first_id = first.id.to_string();
            window.set_active_tunnel_id(first_id.clone().into());
            if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&first_id) {
                let (traffic_in, traffic_out) = tun.formatted_traffic();
                let ssh_cmd = tun.generate_ssh_command(&tun.ssh_host_name, "root");

                window.set_tunnel_form_id(tun.id.into());
                window.set_tunnel_form_name(tun.name.into());
                window.set_tunnel_form_type(tun.tunnel_type.as_str().into());
                window.set_tunnel_form_ssh_host_id(tun.ssh_host_id.unwrap_or_default().into());
                window.set_tunnel_form_ssh_host_name(tun.ssh_host_name.into());
                window.set_tunnel_form_local_bind(tun.local_bind.into());
                window.set_tunnel_form_local_port(tun.local_port.to_string().into());
                window.set_tunnel_form_remote_host(tun.remote_host.into());
                window.set_tunnel_form_remote_port(tun.remote_port.to_string().into());
                window.set_tunnel_form_auto_start(tun.auto_start);
                window.set_tunnel_form_auto_reconnect(tun.auto_reconnect);
                window.set_tunnel_form_remote_dns(tun.remote_dns);
                window.set_tunnel_form_compression(tun.compression);
                window.set_tunnel_form_is_running(tun.is_running);
                window.set_tunnel_form_active_connections(tun.active_connections as i32);
                window.set_tunnel_form_traffic_in(traffic_in.into());
                window.set_tunnel_form_traffic_out(traffic_out.into());
                window.set_tunnel_form_updated_at(tun.updated_at.into());
                window.set_tunnel_form_ssh_command(ssh_cmd.into());
                window.set_tunnel_form_notes(tun.notes.into());
                let hops: Vec<JumpHopData> = tun.jump_chain.iter().map(|h| {
                    JumpHopData {
                        host_id: h.host_id.clone().into(),
                        host_name: h.host_name.clone().into(),
                        host_address: h.host_address.clone().into(),
                        host_port: h.host_port as i32,
                        enabled: h.enabled,
                    }
                }).collect();
                window.set_tunnel_form_jump_chain(ModelRc::new(VecModel::from(hops)));
                window.set_tunnel_form_proxy_proto(if tun.proxy_proto.is_empty() { "SOCKS5".into() } else { tun.proxy_proto.clone().into() });
                window.set_tunnel_form_proxy_username(tun.proxy_username.clone().into());
                window.set_tunnel_form_proxy_password(tun.proxy_password.clone().into());
            }
        }
    } else {
        // 当前查看的规则仍在列表中，同步更新其运行态与流量等实时信息，确保状态立即可见
        if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&current_id) {
            window.set_tunnel_form_is_running(tun.is_running);
            window.set_tunnel_form_active_connections(tun.active_connections as i32);
            let (traffic_in, traffic_out) = tun.formatted_traffic();
            window.set_tunnel_form_traffic_in(traffic_in.into());
            window.set_tunnel_form_traffic_out(traffic_out.into());
        }
    }
}

/// 同步当前活动终端主机专属的端口转发规则至右侧工具栏抽屉
pub(crate) fn sync_ui_host_tunnels(window: &AppWindow, ctx: &AppContext) {
    let host_id = window.get_active_host_id().to_string();
    let host_name = window.get_active_host_name().to_string();
    let sess_name = window.get_active_session_name().to_string();

    let all_tunnels = ctx.core_state.storage().tunnels().list_all().unwrap_or_default();
    let host_tunnels: Vec<TunnelItemData> = all_tunnels
        .into_iter()
        .filter(|t| {
            // 仅端口转发 (Local / Remote / Dynamic / ReverseDynamic)，跳板与代理不作为主机的本地转发
            let is_forward = matches!(t.tunnel_type, TunnelType::Local | TunnelType::Remote | TunnelType::Dynamic | TunnelType::ReverseDynamic);
            if !is_forward {
                return false;
            }
            if !host_id.is_empty() && t.ssh_host_id.as_deref() == Some(&host_id) {
                return true;
            }
            if !host_name.is_empty() && t.ssh_host_name.eq_ignore_ascii_case(&host_name) {
                return true;
            }
            if !sess_name.is_empty() && (t.ssh_host_name.eq_ignore_ascii_case(&sess_name) || sess_name.contains(&t.ssh_host_name)) {
                return true;
            }
            // 如果未关联任何特定主机（例如本地终端或会话未开），也可以展示全部端口转发规则方便测试
            if host_id.is_empty() && host_name.is_empty() && sess_name.is_empty() {
                return true;
            }
            false
        })
        .map(|t| {
            let (traffic_in, traffic_out) = t.formatted_traffic();
            let host_addr = t.ssh_host_name.clone();
            let ssh_cmd = t.generate_ssh_command(&host_addr, "root");
            let route_summary = t.route_summary();

            TunnelItemData {
                id: t.id.into(),
                name: t.name.into(),
                tunnel_type: t.tunnel_type.as_str().into(),
                type_badge: t.tunnel_type.display_badge().into(),
                ssh_host_id: t.ssh_host_id.unwrap_or_default().into(),
                ssh_host_name: t.ssh_host_name.into(),
                local_bind: t.local_bind.into(),
                local_port: t.local_port as i32,
                remote_host: t.remote_host.into(),
                remote_port: t.remote_port as i32,
                route_summary: route_summary.into(),
                is_running: t.is_running,
                auto_start: t.auto_start,
                auto_reconnect: t.auto_reconnect,
                remote_dns: t.remote_dns,
                compression: t.compression,
                active_connections: t.active_connections as i32,
                traffic_in: traffic_in.into(),
                traffic_out: traffic_out.into(),
                notes: t.notes.into(),
                updated_at: t.updated_at.into(),
                ssh_command: ssh_cmd.into(),
            }
        })
        .collect();

    window.set_host_tunnels(ModelRc::new(VecModel::from(host_tunnels)));
}

/// 注册所有网络隧道相关 UI 回调
pub(crate) fn register_tunnel_handlers(window: &AppWindow, ctx: &AppContext) {
    // 0. 同步主机专属隧道列表
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_sync_host_tunnels(move || {
            if let Some(w) = w_handle.upgrade() {
                sync_ui_host_tunnels(&w, &ctx);
            }
        });
    }

    // 1. 选中隧道规则 (查看详情)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_select_tunnel(move |id| {
            if let Some(w) = w_handle.upgrade() {
                w.set_active_tunnel_id(id.clone());
                w.set_is_tunnel_create_mode(false);
                w.set_is_tunnel_editing(false);

                if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&id) {
                    let (traffic_in, traffic_out) = tun.formatted_traffic();
                    let ssh_cmd = tun.generate_ssh_command(&tun.ssh_host_name, "root");

                    w.set_tunnel_form_id(tun.id.into());
                    w.set_tunnel_form_name(tun.name.into());
                    w.set_tunnel_form_type(tun.tunnel_type.as_str().into());
                    w.set_tunnel_form_ssh_host_id(tun.ssh_host_id.unwrap_or_default().into());
                    w.set_tunnel_form_ssh_host_name(tun.ssh_host_name.into());
                    w.set_tunnel_form_local_bind(tun.local_bind.into());
                    w.set_tunnel_form_local_port(tun.local_port.to_string().into());
                    w.set_tunnel_form_remote_host(tun.remote_host.into());
                    w.set_tunnel_form_remote_port(tun.remote_port.to_string().into());
                    w.set_tunnel_form_auto_start(tun.auto_start);
                    w.set_tunnel_form_auto_reconnect(tun.auto_reconnect);
                    w.set_tunnel_form_remote_dns(tun.remote_dns);
                    w.set_tunnel_form_compression(tun.compression);
                    w.set_tunnel_form_is_running(tun.is_running);
                    w.set_tunnel_form_active_connections(tun.active_connections as i32);
                    w.set_tunnel_form_traffic_in(traffic_in.into());
                    w.set_tunnel_form_traffic_out(traffic_out.into());
                    w.set_tunnel_form_updated_at(tun.updated_at.into());
                    w.set_tunnel_form_ssh_command(ssh_cmd.into());
                    w.set_tunnel_form_notes(tun.notes.into());
                    let hops: Vec<JumpHopData> = tun.jump_chain.iter().map(|h| {
                        JumpHopData {
                            host_id: h.host_id.clone().into(),
                            host_name: h.host_name.clone().into(),
                            host_address: h.host_address.clone().into(),
                            host_port: h.host_port as i32,
                            enabled: h.enabled,
                        }
                    }).collect();
                    w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::from(hops)));
                    w.set_tunnel_form_proxy_proto(if tun.proxy_proto.is_empty() { "SOCKS5".into() } else { tun.proxy_proto.clone().into() });
                    w.set_tunnel_form_proxy_username(tun.proxy_username.clone().into());
                    w.set_tunnel_form_proxy_password(tun.proxy_password.clone().into());
                }
            }
        });
    }

    // 2. 切换隧道运行状态 (启动/停止)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_toggle_tunnel_running(move |id| {
            let id_str = id.to_string();
            if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&id_str) {
                let target_state = !tun.is_running;
                let _ = ctx.core_state.storage().tunnels().set_running(&id_str, target_state);

                ctx.core_state.events().dispatch(&TunnelStateChangedEvent {
                    tunnel_id: id_str.clone(),
                    is_running: target_state,
                });

                if target_state {
                    ctx.notify_success("隧道已启动", format!("'{}' 网络连接已建立并监听端口", tun.name));
                } else {
                    ctx.notify_info("隧道已关闭", format!("'{}' 连接已断开，释放本地端口", tun.name));
                }

                if let Some(w) = w_handle.upgrade() {
                    let active_id = w.get_active_tunnel_id().to_string();
                    let form_id = w.get_tunnel_form_id().to_string();
                    if active_id == id_str || form_id == id_str {
                        w.set_tunnel_form_is_running(target_state);
                    }
                    sync_ui_tunnels(&w, &ctx);
                    sync_ui_host_tunnels(&w, &ctx);
                }
            }
        });
    }

    // 3. 开始新建规则
    {
        let w_handle = window.as_weak();
        window.on_start_create_tunnel(move || {
            if let Some(w) = w_handle.upgrade() {
                let new_id = format!("tun-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                w.set_is_tunnel_create_mode(true);
                w.set_is_tunnel_editing(true);
                w.set_active_tunnel_id(new_id.clone().into());
                w.set_tunnel_form_id(new_id.into());
                w.set_tunnel_form_name("未命名端口转发规则".into());
                w.set_tunnel_form_type("Local".into());
                w.set_tunnel_form_ssh_host_id("".into());
                w.set_tunnel_form_ssh_host_name("".into());
                w.set_tunnel_form_local_bind("127.0.0.1".into());
                w.set_tunnel_form_local_port("3306".into());
                w.set_tunnel_form_remote_host("10.0.0.8".into());
                w.set_tunnel_form_remote_port("3306".into());
                w.set_tunnel_form_auto_start(false);
                w.set_tunnel_form_auto_reconnect(true);
                w.set_tunnel_form_remote_dns(false);
                w.set_tunnel_form_compression(true);
                w.set_tunnel_form_is_running(false);
                w.set_tunnel_form_active_connections(0);
                w.set_tunnel_form_traffic_in("0 B".into());
                w.set_tunnel_form_traffic_out("0 B".into());
                w.set_tunnel_form_updated_at("未保存".into());
                w.set_tunnel_form_ssh_command("".into());
                w.set_tunnel_form_notes("".into());
                w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::default()));
                w.set_tunnel_form_proxy_proto("SOCKS5".into());
                w.set_tunnel_form_proxy_username("".into());
                w.set_tunnel_form_proxy_password("".into());
            }
        });
    }

    // 3.B 开始新建指定类型的网络规则 (端口转发 / 跳板机 / 出网代理)
    {
        let w_handle = window.as_weak();
        window.on_start_create_typed_tunnel(move |target_type| {
            if let Some(w) = w_handle.upgrade() {
                let new_id = format!("tun-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                w.set_is_tunnel_create_mode(true);
                w.set_is_tunnel_editing(true);
                w.set_active_tunnel_id(new_id.clone().into());
                w.set_tunnel_form_id(new_id.into());
                w.set_tunnel_form_is_running(false);
                w.set_tunnel_form_active_connections(0);
                w.set_tunnel_form_traffic_in("0 B".into());
                w.set_tunnel_form_traffic_out("0 B".into());
                w.set_tunnel_form_updated_at("未保存".into());
                w.set_tunnel_form_ssh_command("".into());
                w.set_tunnel_form_notes("".into());
                w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::default()));
                w.set_tunnel_form_proxy_proto("SOCKS5".into());
                w.set_tunnel_form_proxy_username("".into());
                w.set_tunnel_form_proxy_password("".into());

                match target_type.as_str() {
                    "JumpHost" => {
                        w.set_tunnel_form_name("未命名跳板链路".into());
                        w.set_tunnel_form_type("JumpHost".into());
                        w.set_tunnel_form_ssh_host_id("".into());
                        w.set_tunnel_form_ssh_host_name("".into());
                        w.set_tunnel_form_local_bind("".into());
                        w.set_tunnel_form_local_port("0".into());
                        w.set_tunnel_form_remote_host("".into());
                        w.set_tunnel_form_remote_port("0".into());
                        w.set_tunnel_form_auto_start(false);
                        w.set_tunnel_form_auto_reconnect(false);
                        w.set_tunnel_form_remote_dns(false);
                        w.set_tunnel_form_compression(false);
                        w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::default()));
                        w.set_tunnel_form_ssh_command("-J <尚未选择启用跳板节点>".into());
                    }
                    "ProxyServer" => {
                        w.set_tunnel_form_name("未命名出网代理".into());
                        w.set_tunnel_form_type("ProxyServer".into());
                        w.set_tunnel_form_ssh_host_id("SOCKS5".into());
                        w.set_tunnel_form_ssh_host_name("SOCKS5".into());
                        w.set_tunnel_form_proxy_proto("SOCKS5".into());
                        w.set_tunnel_form_proxy_username("".into());
                        w.set_tunnel_form_proxy_password("".into());
                        w.set_tunnel_form_local_bind("".into());
                        w.set_tunnel_form_local_port("0".into());
                        w.set_tunnel_form_remote_host("127.0.0.1".into());
                        w.set_tunnel_form_remote_port("7890".into());
                        w.set_tunnel_form_auto_start(false);
                        w.set_tunnel_form_auto_reconnect(false);
                        w.set_tunnel_form_remote_dns(true);
                        w.set_tunnel_form_compression(false);
                        w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::default()));
                        w.set_tunnel_form_ssh_command("ALL_PROXY=socks5://127.0.0.1:7890".into());
                    }
                    _ => {
                        w.set_tunnel_form_name("未命名端口转发规则".into());
                        w.set_tunnel_form_type("Local".into());
                        w.set_tunnel_form_ssh_host_id("".into());
                        w.set_tunnel_form_ssh_host_name("".into());
                        w.set_tunnel_form_local_bind("127.0.0.1".into());
                        w.set_tunnel_form_local_port("3306".into());
                        w.set_tunnel_form_remote_host("10.0.0.8".into());
                        w.set_tunnel_form_remote_port("3306".into());
                        w.set_tunnel_form_auto_start(false);
                        w.set_tunnel_form_auto_reconnect(true);
                        w.set_tunnel_form_remote_dns(false);
                        w.set_tunnel_form_compression(true);
                    }
                }
            }
        });
    }

    // 4. 取消新建
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_cancel_create_tunnel(move || {
            if let Some(w) = w_handle.upgrade() {
                w.set_is_tunnel_create_mode(false);
                w.set_is_tunnel_editing(false);
                let all = ctx.core_state.storage().tunnels().list_all().unwrap_or_default();
                if let Some(first) = all.first() {
                    w.set_active_tunnel_id(first.id.clone().into());
                } else {
                    w.set_active_tunnel_id("".into());
                }
            }
        });
    }

    // 5. 开始编辑
    {
        let w_handle = window.as_weak();
        window.on_start_edit_tunnel(move || {
            if let Some(w) = w_handle.upgrade() {
                w.set_is_tunnel_editing(true);
            }
        });
    }

    // 6. 取消编辑
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_cancel_edit_tunnel(move || {
            if let Some(w) = w_handle.upgrade() {
                w.set_is_tunnel_editing(false);
                let id = w.get_active_tunnel_id().to_string();
                if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&id) {
                    w.set_tunnel_form_name(tun.name.into());
                    w.set_tunnel_form_local_port(tun.local_port.to_string().into());
                    w.set_tunnel_form_remote_port(tun.remote_port.to_string().into());
                }
            }
        });
    }

    // 7. 保存规则 (新建或修改)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_save_tunnel(move |id, name, t_type, h_id, h_name, l_bind, l_port, r_host, r_port, auto_s, auto_r, r_dns, comp, notes| {
            let id_str = id.to_string();
            let name_str = name.to_string();
            if name_str.trim().is_empty() {
                ctx.notify_warning("保存失败", "规则名称不能为空");
                return;
            }

            let is_new = ctx.core_state.storage().tunnels().get_by_id(&id_str).ok().flatten().is_none();
            let parsed_type: TunnelType = t_type.as_str().parse().unwrap_or(TunnelType::Local);

            let (jump_hops, final_remote_host) = if parsed_type == TunnelType::JumpHost {
                if let Some(w) = w_handle.upgrade() {
                    let current_model = w.get_tunnel_form_jump_chain();
                    let hops: Vec<smagical_core::domain::tunnel::JumpHopRecord> = (0..current_model.row_count())
                        .filter_map(|i| current_model.row_data(i))
                        .map(|h| smagical_core::domain::tunnel::JumpHopRecord {
                            host_id: h.host_id.to_string(),
                            host_name: h.host_name.to_string(),
                            host_address: h.host_address.to_string(),
                            host_port: h.host_port as u16,
                            enabled: h.enabled,
                        })
                        .collect();
                    let route_str = hops.iter().filter(|h| h.enabled).map(|h| {
                        if h.host_port == 22 || h.host_port == 0 {
                            h.host_address.clone()
                        } else {
                            format!("{}:{}", h.host_address, h.host_port)
                        }
                    }).collect::<Vec<_>>().join(",");
                    (hops, route_str)
                } else {
                    (Vec::new(), r_host.to_string())
                }
            } else {
                (Vec::new(), r_host.to_string())
            };

            let (proxy_proto, proxy_username, proxy_password) = if parsed_type == TunnelType::ProxyServer {
                if let Some(w) = w_handle.upgrade() {
                    let proto = w.get_tunnel_form_proxy_proto().to_string();
                    let user = w.get_tunnel_form_proxy_username().to_string();
                    let pass = w.get_tunnel_form_proxy_password().to_string();
                    (if proto.is_empty() { "SOCKS5".to_string() } else { proto }, user, pass)
                } else {
                    ("SOCKS5".to_string(), String::new(), String::new())
                }
            } else {
                (String::new(), String::new(), String::new())
            };

            let record = TunnelRecord {
                id: id_str.clone(),
                name: name_str.clone(),
                tunnel_type: parsed_type,
                ssh_host_id: if h_id.trim().is_empty() { None } else { Some(h_id.to_string()) },
                ssh_host_name: h_name.to_string(),
                local_bind: if l_bind.trim().is_empty() { "127.0.0.1".to_string() } else { l_bind.to_string() },
                local_port: l_port as u16,
                remote_host: final_remote_host,
                remote_port: r_port as u16,
                jump_chain: jump_hops,
                is_running: false,
                auto_start: auto_s,
                auto_reconnect: auto_r,
                remote_dns: r_dns,
                compression: comp,
                active_connections: 0,
                total_bytes_in: 0,
                total_bytes_out: 0,
                proxy_proto,
                proxy_username,
                proxy_password,
                notes: notes.to_string(),
                updated_at: "刚刚".to_string(),
            };

            let jump_host_ids: Vec<String> = record.jump_chain.iter().map(|h| h.host_id.clone()).collect();
            let before_save = TunnelBeforeSaveEvent::new(
                &record.id,
                &record.name,
                record.tunnel_type.as_str(),
                &record.local_bind,
                record.local_port,
                &record.remote_host,
                record.remote_port,
                jump_host_ids,
            );
            ctx.core_state.events().dispatch(&before_save);
            if before_save.is_aborted() {
                ctx.notify_warning("配置拦截", before_save.abort_reason().unwrap_or_else(|| "规则前置校验未通过".to_string()));
                return;
            }

            if let Ok(()) = ctx.core_state.storage().tunnels().save(&record) {
                ctx.core_state.events().dispatch(&TunnelSavedEvent {
                    tunnel_id: id_str.clone(),
                    name: name_str,
                    tunnel_type: parsed_type.to_string(),
                    is_new,
                });

                ctx.notify_success("保存成功", "网络隧道配置已持久化");

                if let Some(w) = w_handle.upgrade() {
                    w.set_is_tunnel_create_mode(false);
                    w.set_is_tunnel_editing(false);
                    w.set_active_tunnel_id(id_str.into());
                    sync_ui_tunnels(&w, &ctx);
                    sync_ui_host_tunnels(&w, &ctx);
                }
            }
        });
    }

    // 8. 删除规则
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_delete_tunnel(move |id| {
            let id_str = id.to_string();

            // 1. 删除前审查守卫 (运行中的规则禁止误删)
            let is_running = ctx.core_state.storage().tunnels().get_by_id(&id_str)
                .ok().flatten().map(|t| t.is_running).unwrap_or(false);
            let before_del = TunnelBeforeDeleteEvent::new(&id_str, is_running);
            ctx.core_state.events().dispatch(&before_del);
            if before_del.is_aborted() {
                ctx.notify_warning("删除拦截", before_del.abort_reason().unwrap_or_else(|| "运行中的规则禁止删除".to_string()));
                return;
            }

            if let Ok(true) = ctx.core_state.storage().tunnels().delete(&id_str) {
                ctx.core_state.events().dispatch(&TunnelDeletedEvent {
                    tunnel_id: id_str,
                });
                ctx.notify_success("删除成功", "已从网络配置库中移除该规则");

                if let Some(w) = w_handle.upgrade() {
                    let all = ctx.core_state.storage().tunnels().list_all().unwrap_or_default();
                    if let Some(first) = all.first() {
                        w.set_active_tunnel_id(first.id.clone().into());
                    } else {
                        w.set_active_tunnel_id("".into());
                    }
                    sync_ui_tunnels(&w, &ctx);
                    sync_ui_host_tunnels(&w, &ctx);
                }
            }
        });
    }

    // 9. 复制原生 OpenSSH 命令
    {
        let ctx = ctx.clone();
        window.on_copy_tunnel_ssh_command(move |id| {
            if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&id) {
                let cmd = tun.generate_ssh_command(&tun.ssh_host_name, "root");
                if let Ok(mut clip) = arboard::Clipboard::new() {
                    let _ = clip.set_text(&cmd);
                    ctx.notify_success("复制成功", format!("已将 '{}' 的 SSH 命令复制到剪贴板", tun.name));
                }
            }
        });
    }

    // 10. 过滤检索与分类切换
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_filter_tunnels(move |cat, query| {
            *ctx.tunnel_filter_category.borrow_mut() = cat.to_string();
            *ctx.tunnel_search_query.borrow_mut() = query.to_string();
            if let Some(w) = w_handle.upgrade() {
                w.set_tunnel_filter_category(cat.clone());
                w.set_tunnel_search_query(query.clone());
                sync_ui_tunnels(&w, &ctx);
            }
        });
    }

    // 11. 打开网络管理中心
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        window.on_open_tunnels_hub(move || {
            if let Some(w) = w_handle.upgrade() {
                w.set_main_view("tunnels".into());
                w.set_is_left_drawer_open(false);
                sync_ui_tunnels(&w, &ctx);
            }
        });
    }

    // 11.B 跳板链路节点操作回调 (添加/删除/调序/启停)
    {
        let w_handle = window.as_weak();
        window.on_add_tunnel_jump_hop(move |id, name, addr, port| {
            if let Some(w) = w_handle.upgrade() {
                let current_model = w.get_tunnel_form_jump_chain();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                hops.push(JumpHopData {
                    host_id: id,
                    host_name: name,
                    host_address: addr,
                    host_port: port,
                    enabled: true,
                });
                update_jump_command_preview(&w, &hops);
                w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::from(hops)));
            }
        });
    }

    {
        let w_handle = window.as_weak();
        window.on_remove_tunnel_jump_hop(move |idx| {
            if let Some(w) = w_handle.upgrade() {
                let current_model = w.get_tunnel_form_jump_chain();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                if idx >= 0 && (idx as usize) < hops.len() {
                    hops.remove(idx as usize);
                    update_jump_command_preview(&w, &hops);
                    w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::from(hops)));
                }
            }
        });
    }

    {
        let w_handle = window.as_weak();
        window.on_move_tunnel_jump_hop_up(move |idx| {
            if let Some(w) = w_handle.upgrade() {
                let current_model = w.get_tunnel_form_jump_chain();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                if idx > 0 && (idx as usize) < hops.len() {
                    hops.swap((idx - 1) as usize, idx as usize);
                    update_jump_command_preview(&w, &hops);
                    w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::from(hops)));
                }
            }
        });
    }

    {
        let w_handle = window.as_weak();
        window.on_move_tunnel_jump_hop_down(move |idx| {
            if let Some(w) = w_handle.upgrade() {
                let current_model = w.get_tunnel_form_jump_chain();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                if idx >= 0 && ((idx + 1) as usize) < hops.len() {
                    hops.swap(idx as usize, (idx + 1) as usize);
                    update_jump_command_preview(&w, &hops);
                    w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::from(hops)));
                }
            }
        });
    }

    {
        let w_handle = window.as_weak();
        window.on_toggle_tunnel_jump_hop_enabled(move |idx| {
            if let Some(w) = w_handle.upgrade() {
                let current_model = w.get_tunnel_form_jump_chain();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                if idx >= 0 && (idx as usize) < hops.len() {
                    hops[idx as usize].enabled = !hops[idx as usize].enabled;
                    update_jump_command_preview(&w, &hops);
                    w.set_tunnel_form_jump_chain(ModelRc::new(VecModel::from(hops)));
                }
            }
        });
    }

    // 12. 初始化同步当前主机专属隧道
    sync_ui_host_tunnels(window, ctx);
}
