//! 网络隧道、跳板机与代理管理中心业务回调处理器。
//!
//! 负责规则多维过滤、启停控制、实时拓扑与指标监控、配置保存与删除、原生 OpenSSH 命令生成与复制。
//! 全面采用 TunnelsBridge 领域总线直连架构。

use slint::{ComponentHandle, Model, ModelRc, VecModel};
use smagical_core::domain::tunnel::{TunnelRecord, TunnelRunMode, TunnelType};
use smagical_core::event::{
    TunnelBeforeDeleteEvent, TunnelBeforeSaveEvent, TunnelDeletedEvent, TunnelSavedEvent,
    TunnelStateChangedEvent,
};

use crate::generated::{AppWindow, JumpHopData, TerminalBridge, TunnelItemData, TunnelsBridge};
use crate::handlers::AppContext;
use crate::tunnel_daemon::TunnelDaemonService;

fn update_jump_command_preview(tb: &TunnelsBridge, hops: &[JumpHopData]) {
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
    tb.set_form_ssh_command(cmd.into());
}

fn convert_tunnel_to_item_data(t: &TunnelRecord) -> TunnelItemData {
    let (traffic_in, traffic_out) = t.formatted_traffic();
    let host_addr = t.ssh_host_name.clone();
    let ssh_cmd = t.generate_ssh_command(&host_addr, "root");
    let route_summary = t.route_summary();

    let (status_text, status_badge) = if t.is_running {
        ("运行中", "running")
    } else if t.enabled && t.run_mode == smagical_core::domain::tunnel::TunnelRunMode::FollowTerminal {
        ("伴随终端待命", "standby")
    } else {
        ("已停止", "stopped")
    };

    TunnelItemData {
        id: t.id.clone().into(),
        name: t.name.clone().into(),
        tunnel_type: t.tunnel_type.as_str().into(),
        type_badge: t.tunnel_type.display_badge().into(),
        ssh_host_id: t.ssh_host_id.clone().unwrap_or_default().into(),
        ssh_host_name: t.ssh_host_name.clone().into(),
        local_bind: t.local_bind.clone().into(),
        local_port: t.local_port as i32,
        remote_host: t.remote_host.clone().into(),
        remote_port: t.remote_port as i32,
        route_summary: route_summary.into(),
        is_running: t.is_running,
        enabled: t.enabled,
        run_mode: t.run_mode.as_str().into(),
        status_text: status_text.into(),
        status_badge: status_badge.into(),
        auto_start: t.auto_start,
        auto_reconnect: t.auto_reconnect,
        remote_dns: t.remote_dns,
        compression: t.compression,
        active_connections: t.active_connections as i32,
        traffic_in: traffic_in.into(),
        traffic_out: traffic_out.into(),
        notes: t.notes.clone().into(),
        updated_at: t.updated_at.clone().into(),
        ssh_command: ssh_cmd.into(),
    }
}

/// 将网络隧道规则详情同步回显至 TunnelsBridge 表单状态
pub(crate) fn load_tunnel_into_bridge(tb: &TunnelsBridge, tun: &TunnelRecord) {
    let (traffic_in, traffic_out) = tun.formatted_traffic();
    let ssh_cmd = tun.generate_ssh_command(&tun.ssh_host_name, "root");

    tb.set_form_id(tun.id.clone().into());
    tb.set_form_name(tun.name.clone().into());
    tb.set_form_type(tun.tunnel_type.as_str().into());
    tb.set_form_ssh_host_id(tun.ssh_host_id.clone().unwrap_or_default().into());
    tb.set_form_ssh_host_name(tun.ssh_host_name.clone().into());
    tb.set_form_local_bind(tun.local_bind.clone().into());
    tb.set_form_local_port(tun.local_port.to_string().into());
    tb.set_form_remote_host(tun.remote_host.clone().into());
    tb.set_form_remote_port(tun.remote_port.to_string().into());
    tb.set_form_enabled(tun.enabled);
    tb.set_form_run_mode(tun.run_mode.as_str().into());
    tb.set_form_auto_start(tun.auto_start);
    tb.set_form_auto_reconnect(tun.auto_reconnect);
    tb.set_form_remote_dns(tun.remote_dns);
    tb.set_form_compression(tun.compression);
    tb.set_form_is_running(tun.is_running);
    tb.set_form_active_connections(tun.active_connections as i32);
    tb.set_form_traffic_in(traffic_in.into());
    tb.set_form_traffic_out(traffic_out.into());
    tb.set_form_updated_at(tun.updated_at.clone().into());
    tb.set_form_ssh_command(ssh_cmd.into());
    tb.set_form_notes(tun.notes.clone().into());
    let hops: Vec<JumpHopData> = tun.jump_chain.iter().map(|h| {
        JumpHopData {
            host_id: h.host_id.clone().into(),
            host_name: h.host_name.clone().into(),
            host_address: h.host_address.clone().into(),
            host_port: h.host_port as i32,
            enabled: h.enabled,
        }
    }).collect();
    tb.set_form_hops(ModelRc::new(VecModel::from(hops)));
    tb.set_form_proxy_proto(if tun.proxy_proto.is_empty() { "SOCKS5".into() } else { tun.proxy_proto.clone().into() });
    tb.set_form_proxy_username(tun.proxy_username.clone().into());
    tb.set_form_proxy_password(tun.proxy_password.clone().into());
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
        .map(|t| convert_tunnel_to_item_data(&t))
        .collect();

    let t_model = ModelRc::new(VecModel::from(filtered.clone()));
    let tb = window.global::<TunnelsBridge>();
    tb.set_filter_category(cat.into());
    tb.set_search_query(query.into());
    tb.set_tunnels(t_model);

    // 如果当前选中的规则不在当前过滤结果列表中，自动选中第一条有效规则并加载其表单详情
    let current_id = tb.get_active_tunnel_id().to_string();
    let contains_current = filtered.iter().any(|t| t.id == current_id);
    if !contains_current {
        if let Some(first) = filtered.first() {
            let first_id = first.id.to_string();
            tb.set_active_tunnel_id(first_id.clone().into());
            if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&first_id) {
                load_tunnel_into_bridge(&tb, &tun);
            }
        }
    } else {
        // 当前查看的规则仍在列表中，同步更新其运行态与流量等实时信息，确保状态立即可见
        if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&current_id) {
            let (traffic_in, traffic_out) = tun.formatted_traffic();
            tb.set_form_is_running(tun.is_running);
            tb.set_form_enabled(tun.enabled);
            tb.set_form_run_mode(tun.run_mode.as_str().into());
            tb.set_form_active_connections(tun.active_connections as i32);
            tb.set_form_traffic_in(traffic_in.into());
            tb.set_form_traffic_out(traffic_out.into());
        }
    }
}

/// 同步当前活动终端主机专属的端口转发规则至右侧工具栏抽屉与 TunnelsBridge
pub(crate) fn sync_ui_host_tunnels(window: &AppWindow, ctx: &AppContext) {
    let term_b = window.global::<TerminalBridge>();
    let host_id = term_b.get_active_host_id().to_string();
    let host_name = term_b.get_active_host_name().to_string();
    let sess_name = term_b.get_active_session_name().to_string();
    let tb = window.global::<TunnelsBridge>();

    // 1. 判定是否为本地终端 (local-* 或 local)：本地终端运行于本机系统环境，不支持且无需端口转发
    let is_local = host_id.starts_with("local-") || host_id == "local";
    tb.set_is_local_terminal(is_local);

    if is_local {
        tb.set_active_host_name(if host_name.is_empty() { "本地终端".into() } else { host_name.into() });
        tb.set_active_host_id(host_id.into());
        tb.set_host_tunnels(ModelRc::default());
        return;
    }

    // 2. 若未连接任何远程主机 (无活动会话)
    if host_id.is_empty() {
        tb.set_active_host_name(if host_name.is_empty() { "未连接主机".into() } else { host_name.into() });
        tb.set_active_host_id("".into());
        tb.set_host_tunnels(ModelRc::default());
        return;
    }

    // 3. 针对远程 SSH 主机会话，查询归属于该主机的端口转发规则
    let all_tunnels = ctx.core_state.storage().tunnels().list_all().unwrap_or_default();
    let host_tunnels: Vec<TunnelItemData> = all_tunnels
        .into_iter()
        .filter(|t| {
            // 仅端口转发 (Local / Remote / Dynamic / ReverseDynamic)，跳板与代理不作为主机的本地转发
            let is_forward = matches!(t.tunnel_type, TunnelType::Local | TunnelType::Remote | TunnelType::Dynamic | TunnelType::ReverseDynamic);
            if !is_forward {
                return false;
            }
            if t.ssh_host_id.as_deref() == Some(&host_id) {
                return true;
            }
            if !host_name.is_empty() && t.ssh_host_name.eq_ignore_ascii_case(&host_name) {
                return true;
            }
            if !sess_name.is_empty() && (t.ssh_host_name.eq_ignore_ascii_case(&sess_name) || sess_name.contains(&t.ssh_host_name)) {
                return true;
            }
            false
        })
        .map(|t| convert_tunnel_to_item_data(&t))
        .collect();

    tb.set_active_host_name(host_name.into());
    tb.set_active_host_id(host_id.into());
    tb.set_host_tunnels(ModelRc::new(VecModel::from(host_tunnels)));
}

/// 注册所有网络隧道相关 UI 回调 (全部挂载至 TunnelsBridge 领域总线)
pub(crate) fn register_tunnel_handlers(window: &AppWindow, ctx: &AppContext) {
    let tb = window.global::<TunnelsBridge>();

    // 0. 同步主机专属隧道列表
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_sync_host_tunnels(move || {
            if let Some(w) = w_handle.upgrade() {
                sync_ui_tunnels(&w, &ctx);
                sync_ui_host_tunnels(&w, &ctx);
            }
        });
    }

    // 0.1 打开主机专属端口转发新建弹窗 (直接弹窗，锁定绑定当前主机)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_open_create_host_tunnel_modal(move || {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let term_b = w.global::<TerminalBridge>();
                let mut host_id = term_b.get_active_host_id().to_string();
                let mut host_name = term_b.get_active_host_name().to_string();

                // 本地终端不支持且没必要使用端口转发
                if host_id.starts_with("local-") || host_id == "local" {
                    ctx.notify_warning("不支持端口转发", "本地终端运行于本机系统环境，无需配置网络端口转发");
                    return;
                }

                if host_name.is_empty() {
                    host_name = tb.get_active_host_name().to_string();
                }
                if host_id.is_empty() {
                    host_id = tb.get_active_host_id().to_string();
                }
                if host_id.is_empty() && !host_name.is_empty() {
                    if let Ok(hosts) = ctx.core_state.storage().hosts().list_all() {
                        if let Some(h) = hosts.iter().find(|h| h.name == host_name) {
                            host_id = h.id.clone();
                        }
                    }
                }

                let new_id = format!("tun-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                tb.set_is_create_mode(true);
                tb.set_is_editing(true);
                tb.set_active_tunnel_id(new_id.clone().into());
                tb.set_form_id(new_id.into());
                tb.set_form_name(if host_name.is_empty() { "端口转发规则".into() } else { format!("{}-转发", host_name).into() });
                tb.set_form_type("Local".into());
                tb.set_form_ssh_host_id(host_id.clone().into());
                tb.set_form_ssh_host_name(if host_name.is_empty() { "当前主机".into() } else { host_name.clone().into() });
                tb.set_active_host_id(host_id.into());
                tb.set_active_host_name(if host_name.is_empty() { "当前主机".into() } else { host_name.into() });
                tb.set_form_local_bind("127.0.0.1".into());
                tb.set_form_local_port("8080".into());
                tb.set_form_remote_host("127.0.0.1".into());
                tb.set_form_remote_port("80".into());
                tb.set_form_run_mode("FollowTerminal".into());
                tb.set_form_enabled(true);
                tb.set_form_is_running(false);
                tb.set_form_auto_start(false);
                tb.set_form_auto_reconnect(true);
                tb.set_form_remote_dns(false);
                tb.set_form_compression(true);
                tb.set_form_notes("".into());
                tb.set_form_active_connections(0);
                tb.set_form_traffic_in("0 B".into());
                tb.set_form_traffic_out("0 B".into());
                tb.set_form_updated_at("未保存".into());
                tb.set_form_hops(ModelRc::new(VecModel::default()));
                tb.set_is_create_host_tunnel_modal_open(true);
            }
        });
    }

    // 0.2 关闭主机专属端口转发新建弹窗
    {
        let w_handle = window.as_weak();
        tb.on_close_create_host_tunnel_modal(move || {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                tb.set_is_create_host_tunnel_modal_open(false);
                tb.set_is_create_mode(false);
                tb.set_is_editing(false);
            }
        });
    }

    // 1. 选中隧道规则 (查看详情)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_select_tunnel(move |id| {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                tb.set_active_tunnel_id(id.clone());
                tb.set_is_create_mode(false);
                tb.set_is_editing(false);

                if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&id) {
                    load_tunnel_into_bridge(&tb, &tun);
                }
            }
        });
    }

    // 2. 切换隧道运行状态 (单 Switch 掌管规则启闭，并根据运行模式启停物理通道)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        let toggle_handler = move |id: slint::SharedString, explicit_target: Option<bool>| {
            let id_str = id.to_string();
            if let Ok(Some(mut tun)) = ctx.core_state.storage().tunnels().get_by_id(&id_str) {
                let target_enabled = explicit_target.unwrap_or(!tun.enabled);
                tun.enabled = target_enabled;

                if target_enabled {
                    // 规则被启用：依据运行策略决定是否立即拉起底层监听
                    match tun.run_mode {
                        TunnelRunMode::FollowApp => {
                            let started = TunnelDaemonService::try_start_tunnel(&ctx.core_state.storage(), &tun);
                            tun.is_running = started;
                            if !started {
                                ctx.notify_warning("启动异常", format!("'{}' 端口 {}:{} 建立失败，保持待命状态", tun.name, tun.local_bind, tun.local_port));
                            }
                        }
                        TunnelRunMode::FollowTerminal => {
                            // 检查关联的主机终端当前是否处于打开/连接状态
                            let is_host_connected = if let Some(ref hid) = tun.ssh_host_id {
                                ctx.pane_groups.borrow().iter().flat_map(|g| g.tabs.iter()).any(|t| &t.host_id == hid)
                            } else {
                                false
                            };

                            if is_host_connected {
                                let started = TunnelDaemonService::try_start_tunnel(&ctx.core_state.storage(), &tun);
                                tun.is_running = started;
                                if !started {
                                    ctx.notify_warning("启动异常", format!("'{}' 端口 {}:{} 建立失败", tun.name, tun.local_bind, tun.local_port));
                                }
                            } else {
                                // 伴随终端模式：主机终端未开，进入待命中 (Standby) 状态
                                tun.is_running = false;
                                let _ = ctx.core_state.storage().tunnels().set_running(&id_str, false);
                            }
                        }
                    }
                } else {
                    // 规则被停用：释放端口，停止监听
                    TunnelDaemonService::stop_tunnel(&ctx.core_state.storage(), &id_str);
                    tun.is_running = false;
                }

                // 保存持久化
                let _ = ctx.core_state.storage().tunnels().save(&tun);

                ctx.core_state.events().dispatch(&TunnelStateChangedEvent {
                    tunnel_id: id_str.clone(),
                    is_running: tun.is_running,
                });

                if let Some(w) = w_handle.upgrade() {
                    let tb = w.global::<TunnelsBridge>();
                    let active_id = tb.get_active_tunnel_id().to_string();
                    let form_id = tb.get_form_id().to_string();
                    if active_id == id_str || form_id == id_str {
                        tb.set_form_enabled(tun.enabled);
                        tb.set_form_is_running(tun.is_running);
                    }
                    sync_ui_tunnels(&w, &ctx);
                    sync_ui_host_tunnels(&w, &ctx);
                }
            }
        };

        let t_toggle = {
            let h = toggle_handler.clone();
            move |id: slint::SharedString| h(id, None)
        };
        let t_start = {
            let h = toggle_handler.clone();
            move |id: slint::SharedString| h(id, Some(true))
        };
        let t_stop = {
            let h = toggle_handler;
            move |id: slint::SharedString| h(id, Some(false))
        };

        tb.on_toggle_tunnel(t_toggle);
        tb.on_start_tunnel(t_start);
        tb.on_stop_tunnel(t_stop);
    }

    // 3. 开始新建规则
    {
        let w_handle = window.as_weak();
        tb.on_start_create(move || {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let new_id = format!("tun-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                tb.set_is_create_mode(true);
                tb.set_is_editing(true);
                tb.set_active_tunnel_id(new_id.clone().into());
                tb.set_form_id(new_id.into());
                tb.set_form_name("未命名端口转发规则".into());
                tb.set_form_type("Local".into());
                tb.set_form_ssh_host_id("".into());
                tb.set_form_ssh_host_name("".into());
                tb.set_form_local_bind("127.0.0.1".into());
                tb.set_form_local_port("3306".into());
                tb.set_form_remote_host("10.0.0.8".into());
                tb.set_form_remote_port("3306".into());
                tb.set_form_enabled(true);
                tb.set_form_run_mode("FollowTerminal".into());
                tb.set_form_auto_start(false);
                tb.set_form_auto_reconnect(true);
                tb.set_form_remote_dns(false);
                tb.set_form_compression(true);
                tb.set_form_is_running(false);
                tb.set_form_active_connections(0);
                tb.set_form_traffic_in("0 B".into());
                tb.set_form_traffic_out("0 B".into());
                tb.set_form_updated_at("未保存".into());
                tb.set_form_ssh_command("".into());
                tb.set_form_notes("".into());
                tb.set_form_hops(ModelRc::new(VecModel::default()));
                tb.set_form_proxy_proto("SOCKS5".into());
                tb.set_form_proxy_username("".into());
                tb.set_form_proxy_password("".into());
            }
        });
    }

    // 3.B 开始新建指定类型的网络规则 (端口转发 / 跳板机 / 出网代理)
    {
        let w_handle = window.as_weak();
        tb.on_create_new_tunnel(move |target_type| {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let new_id = format!("tun-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                tb.set_is_create_mode(true);
                tb.set_is_editing(true);
                tb.set_active_tunnel_id(new_id.clone().into());
                tb.set_form_id(new_id.into());
                tb.set_form_enabled(true);
                tb.set_form_run_mode("FollowTerminal".into());
                tb.set_form_is_running(false);
                tb.set_form_active_connections(0);
                tb.set_form_traffic_in("0 B".into());
                tb.set_form_traffic_out("0 B".into());
                tb.set_form_updated_at("未保存".into());
                tb.set_form_ssh_command("".into());
                tb.set_form_notes("".into());
                tb.set_form_hops(ModelRc::new(VecModel::default()));
                tb.set_form_proxy_proto("SOCKS5".into());
                tb.set_form_proxy_username("".into());
                tb.set_form_proxy_password("".into());

                match target_type.as_str() {
                    "JumpHost" => {
                        tb.set_form_name("未命名跳板链路".into());
                        tb.set_form_type("JumpHost".into());
                        tb.set_form_ssh_host_id("".into());
                        tb.set_form_ssh_host_name("".into());
                        tb.set_form_local_bind("".into());
                        tb.set_form_local_port("0".into());
                        tb.set_form_remote_host("".into());
                        tb.set_form_remote_port("0".into());
                        tb.set_form_run_mode("FollowTerminal".into());
                        tb.set_form_auto_start(false);
                        tb.set_form_auto_reconnect(false);
                        tb.set_form_remote_dns(false);
                        tb.set_form_compression(false);
                        tb.set_form_hops(ModelRc::new(VecModel::default()));
                        tb.set_form_ssh_command("-J <尚未选择启用跳板节点>".into());
                    }
                    "ProxyServer" => {
                        tb.set_form_name("未命名出网代理".into());
                        tb.set_form_type("ProxyServer".into());
                        tb.set_form_ssh_host_id("SOCKS5".into());
                        tb.set_form_ssh_host_name("SOCKS5".into());
                        tb.set_form_proxy_proto("SOCKS5".into());
                        tb.set_form_proxy_username("".into());
                        tb.set_form_proxy_password("".into());
                        tb.set_form_local_bind("".into());
                        tb.set_form_local_port("0".into());
                        tb.set_form_remote_host("127.0.0.1".into());
                        tb.set_form_remote_port("7890".into());
                        tb.set_form_run_mode("FollowApp".into());
                        tb.set_form_auto_start(true);
                        tb.set_form_auto_reconnect(false);
                        tb.set_form_remote_dns(true);
                        tb.set_form_compression(false);
                        tb.set_form_hops(ModelRc::new(VecModel::default()));
                        tb.set_form_ssh_command("ALL_PROXY=socks5://127.0.0.1:7890".into());
                    }
                    _ => {
                        tb.set_form_name("未命名端口转发规则".into());
                        tb.set_form_type("Local".into());
                        tb.set_form_ssh_host_id("".into());
                        tb.set_form_ssh_host_name("".into());
                        tb.set_form_local_bind("127.0.0.1".into());
                        tb.set_form_local_port("3306".into());
                        tb.set_form_remote_host("10.0.0.8".into());
                        tb.set_form_remote_port("3306".into());
                        tb.set_form_run_mode("FollowTerminal".into());
                        tb.set_form_auto_start(false);
                        tb.set_form_auto_reconnect(true);
                        tb.set_form_remote_dns(false);
                        tb.set_form_compression(true);
                    }
                }
            }
        });
    }

    // 4. 取消新建
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_cancel_create(move || {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                tb.set_is_create_mode(false);
                tb.set_is_editing(false);
                let all = ctx.core_state.storage().tunnels().list_all().unwrap_or_default();
                if let Some(first) = all.first() {
                    tb.set_active_tunnel_id(first.id.clone().into());
                    load_tunnel_into_bridge(&tb, first);
                } else {
                    tb.set_active_tunnel_id("".into());
                }
            }
        });
    }

    // 5. 开始编辑
    {
        let w_handle = window.as_weak();
        tb.on_start_edit(move || {
            if let Some(w) = w_handle.upgrade() {
                w.global::<TunnelsBridge>().set_is_editing(true);
            }
        });
    }

    // 6. 取消编辑
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_cancel_edit(move || {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                tb.set_is_editing(false);
                let id = tb.get_active_tunnel_id().to_string();
                if let Ok(Some(tun)) = ctx.core_state.storage().tunnels().get_by_id(&id) {
                    load_tunnel_into_bridge(&tb, &tun);
                }
            }
        });
    }

    // 7. 保存规则 (新建或修改 - 严格遵循“修改时先关闭旧通道、再保存新配置、后根据开关状态重新拉起”的安全热重载生命周期)
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_save_tunnel(move || {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let id_str = tb.get_form_id().to_string();
                let name_str = tb.get_form_name().to_string();
                let t_type = tb.get_form_type().to_string();
                let h_id = tb.get_form_ssh_host_id().to_string();
                let h_name = tb.get_form_ssh_host_name().to_string();
                let l_bind = tb.get_form_local_bind().to_string();
                let l_port = tb.get_form_local_port().parse::<u16>().unwrap_or(0);
                let r_host = tb.get_form_remote_host().to_string();
                let r_port = tb.get_form_remote_port().parse::<u16>().unwrap_or(0);
                let form_enabled = tb.get_form_enabled();
                let form_run_mode_str = tb.get_form_run_mode().to_string();
                let parsed_run_mode: TunnelRunMode = form_run_mode_str.parse().unwrap_or(TunnelRunMode::FollowTerminal);
                let auto_s = parsed_run_mode == TunnelRunMode::FollowApp;
                let auto_r = tb.get_form_auto_reconnect();
                let r_dns = tb.get_form_remote_dns();
                let comp = tb.get_form_compression();
                let notes = tb.get_form_notes().to_string();

                if name_str.trim().is_empty() {
                    ctx.notify_warning("保存失败", "规则名称不能为空");
                    return;
                }

                // 检查旧配置是否存在，且是否正在运行
                let old_record = ctx.core_state.storage().tunnels().get_by_id(&id_str).ok().flatten();
                let is_new = old_record.is_none();

                // 【核心要求：修改时先关闭旧通道】
                if let Some(ref old_tun) = old_record {
                    if old_tun.is_running {
                        TunnelDaemonService::stop_tunnel(&ctx.core_state.storage(), &old_tun.id);
                        ctx.core_state.events().dispatch(&TunnelStateChangedEvent {
                            tunnel_id: old_tun.id.clone(),
                            is_running: false,
                        });
                        tracing::info!(
                            target: "smalux::tunnel",
                            "热重载：编辑修改前已先关闭旧通道 [{}] 释放端口 {}:{}",
                            old_tun.id, old_tun.local_bind, old_tun.local_port
                        );
                    }
                }

                let parsed_type: TunnelType = t_type.as_str().parse().unwrap_or(TunnelType::Local);

                let (jump_hops, final_remote_host) = if parsed_type == TunnelType::JumpHost {
                    let current_model = tb.get_form_hops();
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
                    (Vec::new(), r_host)
                };

                let (proxy_proto, proxy_username, proxy_password) = if parsed_type == TunnelType::ProxyServer {
                    let proto = tb.get_form_proxy_proto().to_string();
                    let user = tb.get_form_proxy_username().to_string();
                    let pass = tb.get_form_proxy_password().to_string();
                    (if proto.is_empty() { "SOCKS5".to_string() } else { proto }, user, pass)
                } else {
                    (String::new(), String::new(), String::new())
                };

                let mut record = TunnelRecord {
                    id: id_str.clone(),
                    name: name_str.clone(),
                    tunnel_type: parsed_type,
                    ssh_host_id: if h_id.trim().is_empty() { None } else { Some(h_id) },
                    ssh_host_name: h_name,
                    local_bind: if l_bind.trim().is_empty() { "127.0.0.1".to_string() } else { l_bind },
                    local_port: l_port,
                    remote_host: final_remote_host,
                    remote_port: r_port,
                    jump_chain: jump_hops,
                    enabled: form_enabled,
                    is_running: false,
                    run_mode: parsed_run_mode,
                    auto_start: auto_s,
                    auto_reconnect: auto_r,
                    remote_dns: r_dns,
                    compression: comp,
                    active_connections: 0,
                    total_bytes_in: old_record.as_ref().map(|o| o.total_bytes_in).unwrap_or(0),
                    total_bytes_out: old_record.as_ref().map(|o| o.total_bytes_out).unwrap_or(0),
                    proxy_proto,
                    proxy_username,
                    proxy_password,
                    notes,
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

                    // 【核心要求：根据更改后的内容和开关状态重新拉起】
                    let mut actually_started = false;
                    if record.enabled {
                        match record.run_mode {
                            TunnelRunMode::FollowApp => {
                                actually_started = TunnelDaemonService::try_start_tunnel(&ctx.core_state.storage(), &record);
                                if actually_started {
                                    record.is_running = true;
                                }
                            }
                            TunnelRunMode::FollowTerminal => {
                                let is_host_connected = if let Some(ref hid) = record.ssh_host_id {
                                    ctx.pane_groups.borrow().iter().flat_map(|g| g.tabs.iter()).any(|t| &t.host_id == hid)
                                } else {
                                    false
                                };
                                if is_host_connected {
                                    actually_started = TunnelDaemonService::try_start_tunnel(&ctx.core_state.storage(), &record);
                                    if actually_started {
                                        record.is_running = true;
                                    }
                                }
                            }
                        }
                    }

                    if record.is_running {
                        ctx.core_state.events().dispatch(&TunnelStateChangedEvent {
                            tunnel_id: id_str.clone(),
                            is_running: true,
                        });
                    }

                    if record.enabled {
                        if actually_started {
                            ctx.notify_success("保存并生效", format!("'{}' 配置已更新，新通道已建立监听", record.name));
                        } else if record.run_mode == TunnelRunMode::FollowTerminal {
                            ctx.notify_success("保存成功", format!("'{}' 配置已更新并处于待命状态，将在打开终端时自动激活", record.name));
                        } else {
                            ctx.notify_warning("配置已保存", format!("'{}' 端口 {}:{} 建立监听失败，请检查端口占用", record.name, record.local_bind, record.local_port));
                        }
                    } else {
                        ctx.notify_info("保存成功", format!("'{}' 规则已保存（处于停用关闭状态）", record.name));
                    }

                    tb.set_is_create_mode(false);
                    tb.set_is_editing(false);
                    tb.set_is_create_host_tunnel_modal_open(false);
                    tb.set_active_tunnel_id(id_str.into());
                    load_tunnel_into_bridge(&tb, &record);
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
        tb.on_delete_tunnel(move |id| {
            let id_str = id.to_string();

            // 删除前审查守卫 (运行中的规则禁止误删)
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
                    let tb = w.global::<TunnelsBridge>();
                    let all = ctx.core_state.storage().tunnels().list_all().unwrap_or_default();
                    if let Some(first) = all.first() {
                        tb.set_active_tunnel_id(first.id.clone().into());
                        load_tunnel_into_bridge(&tb, first);
                    } else {
                        tb.set_active_tunnel_id("".into());
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
        tb.on_copy_ssh_command(move |id| {
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
        tb.on_search_changed(move |query| {
            *ctx.tunnel_search_query.borrow_mut() = query.to_string();
            if let Some(w) = w_handle.upgrade() {
                sync_ui_tunnels(&w, &ctx);
            }
        });
    }
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_filter_changed(move |cat| {
            *ctx.tunnel_filter_category.borrow_mut() = cat.to_string();
            if let Some(w) = w_handle.upgrade() {
                sync_ui_tunnels(&w, &ctx);
            }
        });
    }

    // 11. 复制自定义文本
    {
        let notif_custom_copy = ctx.notifications.clone();
        tb.on_copy_custom_text(move |text, title, msg| {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(text.to_string());
                notif_custom_copy.success(title.as_str(), msg.as_str());
            } else {
                notif_custom_copy.warning("剪贴板受限", "无法访问操作系统剪贴板服务");
            }
        });
    }

    // 12. 跳板链路节点操作回调 (添加/删除/调序/启停)
    {
        let w_handle = window.as_weak();
        tb.on_add_jump_hop(move |id, name, addr, port| {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let current_model = tb.get_form_hops();
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
                update_jump_command_preview(&tb, &hops);
                tb.set_form_hops(ModelRc::new(VecModel::from(hops)));
            }
        });
    }

    {
        let w_handle = window.as_weak();
        tb.on_remove_jump_hop(move |idx| {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let current_model = tb.get_form_hops();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                if idx >= 0 && (idx as usize) < hops.len() {
                    hops.remove(idx as usize);
                    update_jump_command_preview(&tb, &hops);
                    tb.set_form_hops(ModelRc::new(VecModel::from(hops)));
                }
            }
        });
    }

    {
        let w_handle = window.as_weak();
        tb.on_move_jump_hop_up(move |idx| {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let current_model = tb.get_form_hops();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                if idx > 0 && (idx as usize) < hops.len() {
                    hops.swap((idx - 1) as usize, idx as usize);
                    update_jump_command_preview(&tb, &hops);
                    tb.set_form_hops(ModelRc::new(VecModel::from(hops)));
                }
            }
        });
    }

    {
        let w_handle = window.as_weak();
        tb.on_move_jump_hop_down(move |idx| {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let current_model = tb.get_form_hops();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                if idx >= 0 && ((idx + 1) as usize) < hops.len() {
                    hops.swap(idx as usize, (idx + 1) as usize);
                    update_jump_command_preview(&tb, &hops);
                    tb.set_form_hops(ModelRc::new(VecModel::from(hops)));
                }
            }
        });
    }

    {
        let w_handle = window.as_weak();
        tb.on_toggle_jump_hop(move |idx| {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                let current_model = tb.get_form_hops();
                let mut hops: Vec<JumpHopData> = (0..current_model.row_count())
                    .filter_map(|i| current_model.row_data(i))
                    .collect();
                if idx >= 0 && (idx as usize) < hops.len() {
                    hops[idx as usize].enabled = !hops[idx as usize].enabled;
                    update_jump_command_preview(&tb, &hops);
                    tb.set_form_hops(ModelRc::new(VecModel::from(hops)));
                }
            }
        });
    }

    // 13. 初始化同步当前主机专属隧道
    sync_ui_host_tunnels(window, ctx);
}
