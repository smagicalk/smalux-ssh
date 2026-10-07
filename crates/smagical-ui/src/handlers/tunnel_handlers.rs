//! 网络隧道、跳板机与代理管理中心业务回调处理器。
//!
//! 负责规则多维过滤、启停控制、实时拓扑与指标监控、配置保存与删除、原生 OpenSSH 命令生成与复制。
//! 全面采用 TunnelsBridge 领域总线直连架构。

use slint::{ComponentHandle, Model, ModelRc};
use smagical_core::domain::tunnel::{TunnelRecord, TunnelRunMode, TunnelType};
use smagical_core::event::{
    TunnelBeforeDeleteEvent, TunnelBeforeSaveEvent, TunnelDeletedEvent, TunnelSavedEvent,
    TunnelStateChangedEvent,
};

use crate::common::{matches_any_ignore_case, num_to_shared, to_model_rc, ToSharedString};
use crate::generated::{AppWindow, JumpHopData, TerminalBridge, TunnelItemData, TunnelsBridge};
use crate::handlers::AppContext;
use crate::tunnel_daemon::TunnelDaemonService;

/// 实时根据勾选的跳板节点生成原生 OpenSSH 多跳命令行预览。
///
/// # 算法逻辑
/// 1. 过滤出所有 `enabled == true` 的跳板节点；
/// 2. 拼接地址：若端口为 22 则省略，否则追加 `:port`；
/// 3. 用逗号 `,` 串联生成标准 `ssh -J hop1,hop2 target-user@target-host` 命令并回显至表单。
///
/// # 参数
/// - `tb`: Slint TunnelsBridge 领域总线句柄；
/// - `hops`: 当前链路的跳板节点切片。
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

/// 将核心领域模型 [`TunnelRecord`] 转换为 Slint 渲染所需的数据传输模型 [`TunnelItemData`]。
///
/// # 状态推导逻辑
/// - `is_running == true`: 标记为 `"running"`（绿色运行中）；
/// - `enabled == true` 且 `run_mode == FollowTerminal`: 标记为 `"standby"`（黄色伴随终端待命）；
/// - 其余状态: 标记为 `"stopped"`（灰色已停止）。
///
/// # 参数
/// - `t`: 核心领域层隧道数据实体引用。
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
        id: t.id.to_shared(),
        name: t.name.to_shared(),
        tunnel_type: t.tunnel_type.as_str().to_shared(),
        type_badge: t.tunnel_type.display_badge().to_shared(),
        ssh_host_id: t.ssh_host_id.to_shared(),
        ssh_host_name: t.ssh_host_name.to_shared(),
        local_bind: t.local_bind.to_shared(),
        local_port: t.local_port as i32,
        remote_host: t.remote_host.to_shared(),
        remote_port: t.remote_port as i32,
        route_summary: route_summary.into(),
        is_running: t.is_running,
        enabled: t.enabled,
        run_mode: t.run_mode.as_str().to_shared(),
        status_text: status_text.into(),
        status_badge: status_badge.into(),
        auto_start: t.auto_start,
        auto_reconnect: t.auto_reconnect,
        remote_dns: t.remote_dns,
        compression: t.compression,
        active_connections: t.active_connections as i32,
        traffic_in: traffic_in.into(),
        traffic_out: traffic_out.into(),
        notes: t.notes.to_shared(),
        updated_at: t.updated_at.to_shared(),
        ssh_command: ssh_cmd.into(),
    }
}

/// 将网络隧道规则详情同步回显至 TunnelsBridge 表单状态。
///
/// # 属性载入
/// 载入规则基础属性（ID、名称、类型、SSH 主机、端口绑定、运行模式）、
/// 动态流量指标（接收与发送字节量）、多跳跳板链路列表以及代理鉴权账号密码。
///
/// # 参数
/// - `tb`: Slint TunnelsBridge 全局单例句柄；
/// - `tun`: 待回显展示的隧道实体。
pub(crate) fn load_tunnel_into_bridge(tb: &TunnelsBridge, tun: &TunnelRecord) {
    let (traffic_in, traffic_out) = tun.formatted_traffic();
    let ssh_cmd = tun.generate_ssh_command(&tun.ssh_host_name, "root");

    tb.set_form_id(tun.id.to_shared());
    tb.set_form_name(tun.name.to_shared());
    tb.set_form_type(tun.tunnel_type.as_str().to_shared());
    tb.set_form_ssh_host_id(tun.ssh_host_id.to_shared());
    tb.set_form_ssh_host_name(tun.ssh_host_name.to_shared());
    tb.set_form_local_bind(tun.local_bind.to_shared());
    tb.set_form_local_port(num_to_shared(tun.local_port));
    tb.set_form_remote_host(tun.remote_host.to_shared());
    tb.set_form_remote_port(num_to_shared(tun.remote_port));
    tb.set_form_enabled(tun.enabled);
    tb.set_form_run_mode(tun.run_mode.as_str().to_shared());
    tb.set_form_auto_start(tun.auto_start);
    tb.set_form_auto_reconnect(tun.auto_reconnect);
    tb.set_form_remote_dns(tun.remote_dns);
    tb.set_form_compression(tun.compression);
    tb.set_form_is_running(tun.is_running);
    tb.set_form_active_connections(tun.active_connections as i32);
    tb.set_form_traffic_in(traffic_in.into());
    tb.set_form_traffic_out(traffic_out.into());
    tb.set_form_updated_at(tun.updated_at.to_shared());
    tb.set_form_ssh_command(ssh_cmd.into());
    tb.set_form_notes(tun.notes.to_shared());
    let hops: Vec<JumpHopData> = tun.jump_chain.iter().map(|h| {
        JumpHopData {
            host_id: h.host_id.to_shared(),
            host_name: h.host_name.to_shared(),
            host_address: h.host_address.to_shared(),
            host_port: h.host_port as i32,
            enabled: h.enabled,
        }
    }).collect();
    tb.set_form_hops(to_model_rc(hops));
    tb.set_form_proxy_proto(if tun.proxy_proto.is_empty() { "SOCKS5".into() } else { tun.proxy_proto.to_shared() });
    tb.set_form_proxy_username(tun.proxy_username.to_shared());
    tb.set_form_proxy_password(tun.proxy_password.to_shared());
}

/// 纯 UI 渲染函数：根据全量隧道记录列表、分类与搜索关键词，过滤并装载到 Slint `TunnelsBridge`
///
/// # 业务逻辑
/// 1. **双重过滤机制**：
///    - **分类过滤 (`cat`)**：
///      - `"all"`: 包含所有类型规则；
///      - `"forward"`: 端口转发规则（包含 `Local`, `Remote`, `Dynamic`, `ReverseDynamic`）；
///      - `"jump"`: 多跳跳板链路规则（`JumpHost`）；
///      - `"proxy"`: 出网代理服务器规则（`ProxyServer`）。
///    - **关键词模糊匹配 (`query`)**：
///      - 不区分大小写匹配规则名称、远端主机、本地端口、远端端口、关联 SSH 主机名以及备注说明。
/// 2. **自动状态补全与选中焦点维护**：
///    - 若当前活动选中项不在过滤结果中，自动选中第一项并加载其表单详情；
///    - 若当前项仍存在，就地更新其实时运行态（`is_running`、`enabled`、活跃连接数与格式化流量），避免 UI 闪烁。
///
/// # 参数
/// - `window`: Slint 顶级应用窗口上下文句柄；
/// - `all_tunnels`: 从存储层拉取的全量隧道规则镜像切片；
/// - `cat`: 当前选中的分类过滤器标识；
/// - `query`: 搜索框中输入的文本关键词。
pub(crate) fn render_tunnels_ui(window: &AppWindow, all_tunnels: &[TunnelRecord], cat: &str, query: &str) {
    let query_lower = query.trim().to_lowercase();
    let filtered: Vec<TunnelItemData> = all_tunnels
        .iter()
        .filter(|t| {
            // 1. 分类过滤
            let match_cat = match cat {
                "all" => true,
                "forward" => matches!(t.tunnel_type, TunnelType::Local | TunnelType::Remote | TunnelType::Dynamic | TunnelType::ReverseDynamic),
                "jump" => matches!(t.tunnel_type, TunnelType::JumpHost),
                "proxy" => matches!(t.tunnel_type, TunnelType::ProxyServer),
                _ => true,
            };
            if !match_cat {
                return false;
            }

            // 2. 关键词模糊搜索 (零堆分配匹配)
            if query_lower.is_empty() {
                true
            } else {
                let lp = t.local_port.to_string();
                let rp = t.remote_port.to_string();
                matches_any_ignore_case(
                    &[
                        &t.name,
                        &t.remote_host,
                        &lp,
                        &rp,
                        &t.ssh_host_name,
                        &t.notes,
                    ],
                    &query_lower,
                )
            }
        })
        .map(|t| convert_tunnel_to_item_data(t))
        .collect();

    let tb = window.global::<TunnelsBridge>();
    tb.set_filter_category(cat.into());
    tb.set_search_query(query.into());
    let current_tunnels = tb.get_tunnels();
    crate::store::diff::update_model_rc_in_place(&current_tunnels, filtered.clone(), |m| tb.set_tunnels(m));

    // 如果当前选中的规则不在当前过滤结果列表中，自动选中第一条有效规则并加载其表单详情
    let current_id = tb.get_active_tunnel_id().to_string();
    let contains_current = filtered.iter().any(|t| t.id == current_id);
    if !contains_current {
        if let Some(first) = filtered.first() {
            let first_id = first.id.to_string();
            tb.set_active_tunnel_id(first_id.clone().into());
            if let Some(tun) = all_tunnels.iter().find(|t| t.id == first_id) {
                load_tunnel_into_bridge(&tb, tun);
            }
        }
    } else {
        // 当前查看的规则仍在列表中，同步更新其运行态与流量等实时信息，确保状态立即可见
        if let Some(tun) = all_tunnels.iter().find(|t| t.id == current_id) {
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

/// 异步从存储层拉取全量网络隧道记录并调度回主 UI 线程更新 Slint `TunnelsBridge`
///
/// # 参数
/// - `window_weak`: Slint 主窗口弱引用，用于跨异步边界安全升级；
/// - `storage`: 仓储服务抽象接口 `Arc<dyn AppStorage>`；
/// - `cat`: 分类过滤器标识；
/// - `query`: 检索关键词。
pub(crate) fn sync_ui_tunnels_async(
    window_weak: slint::Weak<AppWindow>,
    storage: std::sync::Arc<dyn smagical_core::AppStorage>,
    cat: String,
    query: String,
) {
    crate::async_util::spawn_async(async move {
        let all_tunnels = storage.tunnels().list_all().await.unwrap_or_default();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                render_tunnels_ui(&w, &all_tunnels, &cat, &query);
            }
        });
    });
}

/// 同步更新 Slint 网络隧道与代理规则列表 (非阻塞发起异步查询)
///
/// # 参数
/// - `window`: Slint 顶级应用窗口；
/// - `ctx`: 应用程序全局上下文引用。
pub(crate) fn sync_ui_tunnels(window: &AppWindow, ctx: &AppContext) {
    let window_weak = window.as_weak();
    let storage = ctx.core_state.storage().clone();
    let cat = ctx.tunnel_filter_category.borrow().clone();
    let query = ctx.tunnel_search_query.borrow().clone();
    sync_ui_tunnels_async(window_weak.clone(), storage, cat, query);
    ctx.host_store.schedule_tree_refresh(window_weak);
}

/// 纯 UI 渲染函数：同步当前活动终端主机专属的端口转发规则至右侧工具栏抽屉与 `TunnelsBridge`
///
/// # 业务规则
/// 1. **本地环境保护**：
///    - 若 `active_host_id` 为 `local-*` 或 `local`，表示为本地 PowerShell/CMD/Bash 终端，不支持且无需配置远程端口转发；
/// 2. **远程主机归属匹配**：
///    - 仅过滤出端口转发类型（`Local`, `Remote`, `Dynamic`, `ReverseDynamic`）；
///    - 匹配 `ssh_host_id` 精确对齐，或回退匹配 `ssh_host_name` 忽略大小写；
/// 3. **状态绑定**：
///    - 更新 `TunnelsBridge` 中的 `active_host_name`、`active_host_id` 以及 `host_tunnels` 模型。
///
/// # 参数
/// - `window`: Slint 应用窗口；
/// - `all_tunnels`: 全量隧道记录切片。
pub(crate) fn render_host_tunnels_ui(window: &AppWindow, all_tunnels: &[TunnelRecord]) {
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
    let host_tunnels: Vec<TunnelItemData> = all_tunnels
        .iter()
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
        .map(|t| convert_tunnel_to_item_data(t))
        .collect();

    tb.set_active_host_name(host_name.into());
    tb.set_active_host_id(host_id.into());
    tb.set_host_tunnels(to_model_rc(host_tunnels));
}

/// 异步从存储层拉取隧道记录并同步当前活动终端主机专属的端口转发规则
///
/// # 参数
/// - `window_weak`: Slint 主窗口弱引用；
/// - `storage`: 仓储服务抽象接口。
pub(crate) fn sync_ui_host_tunnels_async(
    window_weak: slint::Weak<AppWindow>,
    storage: std::sync::Arc<dyn smagical_core::AppStorage>,
) {
    crate::async_util::spawn_async(async move {
        let all_tunnels = storage.tunnels().list_all().await.unwrap_or_default();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                render_host_tunnels_ui(&w, &all_tunnels);
            }
        });
    });
}

/// 同步当前活动终端主机专属的端口转发规则至右侧工具栏抽屉与 `TunnelsBridge`
///
/// # 参数
/// - `window`: Slint 应用主窗口；
/// - `ctx`: 全局应用上下文。
pub(crate) fn sync_ui_host_tunnels(window: &AppWindow, ctx: &AppContext) {
    let window_weak = window.as_weak();
    let storage = ctx.core_state.storage().clone();
    sync_ui_host_tunnels_async(window_weak, storage);
}

/// 注册所有网络隧道、跳板机链路与代理服务相关 UI 回调
///
/// 本函数将所有网络规则的前端事件完整挂载至 Slint `TunnelsBridge`，涵盖：
/// 1. **主机专属转发抽屉联动**：实时刷新当前远程会话关联的专属隧道；
/// 2. **规则选中与双向装载**：在主表单与右侧编辑区双向绑定；
/// 3. **启停受控流转**：结合 `TunnelRunMode` 判定伴随应用启动还是伴随终端启动；
/// 4. **热重载保存与生命周期控制**：更新配置前先优雅关闭旧通道释放端口，保存后自动以新配置拉起；
/// 5. **安全删除审查**：拦截正在运行规则的误删行为；
/// 6. **跳板机多跳链路交互**：多级 Jump Host 节点的动态增删、上下调序与启闭；
/// 7. **原生 OpenSSH 命令生成与剪贴板集成**。
///
/// # 参数
/// - `window`: Slint 顶级应用窗口；
/// - `ctx`: 应用程序全局上下文引用。
pub(crate) fn register_tunnel_handlers(window: &AppWindow, ctx: &AppContext) {
    let tb = window.global::<TunnelsBridge>();

    // -------------------------------------------------------------------------
    // 0. 同步主机专属隧道与全量规则列表
    // -------------------------------------------------------------------------
    // 触发场景：终端切换会话、主机连接建立或断开、规则列表需要全量重绘时调用。
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

    // -------------------------------------------------------------------------
    // 0.05 准备主机选择树 (为跳板机或端口转发宿主选择弹窗预加载主机树)
    // -------------------------------------------------------------------------
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_prepare_host_picker(move || {
            ctx.host_store.schedule_tree_refresh(w_handle.clone());
        });
    }

    // -------------------------------------------------------------------------
    // 0.1 打开主机专属端口转发新建弹窗
    // -------------------------------------------------------------------------
    // 触发场景：在右侧工具栏抽屉中点击“为当前主机新建转发规则”。
    // 业务逻辑：
    // 1. 本地终端防御拦截：若当前为本地控制台 (local-*)，直接阻断并弹出告警，防止无意义的网络转发；
    // 2. 自动绑定上下文：解析当前激活的终端标签 `host_id` 与 `host_name`，绑定到表单；
    // 3. 初始模板注入：设置默认端口（127.0.0.1:8080 -> 127.0.0.1:80）、运行模式为 FollowTerminal；
    // 4. 打开 Slint 模态对话框 `is_create_host_tunnel_modal_open`。
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
                    let tree = ctx.master_tree.read().unwrap();
                    if let Some(h) = tree.iter().find(|n| !n.is_group && n.name == host_name) {
                        host_id = h.id.clone();
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
                tb.set_form_hops(ModelRc::default());
                tb.set_is_create_host_tunnel_modal_open(true);
            }
        });
    }

    // -------------------------------------------------------------------------
    // 0.2 关闭主机专属端口转发新建弹窗
    // -------------------------------------------------------------------------
    // 触发场景：用户在主机转发弹窗中点击“取消”或模态框背景遮罩。
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

    // -------------------------------------------------------------------------
    // 1. 选中隧道规则 (查看详情)
    // -------------------------------------------------------------------------
    // 触发场景：在隧道列表中点击某项规则。
    // 业务逻辑：
    // 1. 设置当前活动选中项 ID；
    // 2. 退出新建模式与编辑模式；
    // 3. 异步从 SQLite 存储层拉取实体记录，并调用 `load_tunnel_into_bridge` 回填表单与统计数据。
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_select_tunnel(move |id| {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                tb.set_active_tunnel_id(id.clone());
                tb.set_is_create_mode(false);
                tb.set_is_editing(false);

                let id_str = id.to_string();
                let storage = ctx.core_state.storage();
                let w_weak = w_handle.clone();
                crate::async_util::spawn_async(async move {
                    if let Ok(Some(tun)) = storage.tunnels().get_by_id(&id_str).await {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = w_weak.upgrade() {
                                let tb = w.global::<TunnelsBridge>();
                                load_tunnel_into_bridge(&tb, &tun);
                            }
                        });
                    }
                });
            }
        });
    }

    // -------------------------------------------------------------------------
    // 2. 切换隧道运行状态 (单 Switch 掌管规则启闭，并根据运行模式启停物理通道)
    // -------------------------------------------------------------------------
    // 统一处理 `on_toggle_tunnel`、`on_start_tunnel` 与 `on_stop_tunnel`。
    // 业务状态机流转：
    // - 停用 (`enabled: false`)：
    //   调用 `stop_tunnel` 强制释放物理端口监听，清除运行标志，持久化更新。
    // - 启用 (`enabled: true`)：
    //   - 若为 `FollowApp`（随应用常驻）：立即尝试拉起底层监听；
    //   - 若为 `FollowTerminal`（随终端启动）：
    //     检查其关联的远程主机是否已在当前会话窗格中建立连接。若已连通则立即拉起；若未连通则进入待命 (Standby) 状态。
    // - 状态广播：分发 `TunnelStateChangedEvent`，并以 Toast 提示端口异常与就绪状态。
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        let toggle_handler = move |id: slint::SharedString, explicit_target: Option<bool>| {
            let id_str = id.to_string();
            let storage = ctx.core_state.storage();
            let tunnels_svc = ctx.core_state.tunnels();
            let events = ctx.core_state.events().clone();
            let connected_hosts: std::collections::HashSet<String> = ctx
                .pane_groups
                .borrow()
                .iter()
                .flat_map(|g| g.tabs.iter())
                .map(|t| t.host_id.clone())
                .collect();
            let w_weak = w_handle.clone();

            crate::async_util::spawn_async(async move {
                if let Ok(Some(mut tun)) = storage.tunnels().get_by_id(&id_str).await {
                    let target_enabled = explicit_target.unwrap_or(!tun.enabled);
                    tun.enabled = target_enabled;
                    let mut warn_msg: Option<String> = None;

                    if target_enabled {
                        // 规则被启用：依据运行策略决定是否立即拉起底层监听
                        match tun.run_mode {
                            TunnelRunMode::FollowApp => {
                                match TunnelDaemonService::try_start_tunnel_with_result(&storage, &tunnels_svc, &tun).await {
                                    Ok(_) => {
                                        tun.is_running = true;
                                    }
                                    Err(err) => {
                                        tun.is_running = false;
                                        warn_msg = Some(format!(
                                            "'{}' 端口 {}:{} 启动失败:\n{}",
                                            tun.name, tun.local_bind, tun.local_port, err
                                        ));
                                    }
                                }
                            }
                            TunnelRunMode::FollowTerminal => {
                                // 检查关联的主机终端当前是否处于打开/连接状态
                                let is_host_connected = tun
                                    .ssh_host_id
                                    .as_ref()
                                    .map(|hid| connected_hosts.contains(hid))
                                    .unwrap_or(false);

                                if is_host_connected {
                                    match TunnelDaemonService::try_start_tunnel_with_result(&storage, &tunnels_svc, &tun).await {
                                        Ok(_) => {
                                            tun.is_running = true;
                                        }
                                        Err(err) => {
                                            tun.is_running = false;
                                            warn_msg = Some(format!(
                                                "伴生规则 '{}' 端口 {}:{} 启动失败:\n{}",
                                                tun.name, tun.local_bind, tun.local_port, err
                                            ));
                                        }
                                    }
                                } else {
                                    // 伴随终端模式：主机终端未开，进入待命中 (Standby) 状态
                                    tun.is_running = false;
                                    let _ = storage.tunnels().set_running(&id_str, false).await;
                                }
                            }
                        }
                    } else {
                        // 规则被停用：释放端口，停止监听
                        TunnelDaemonService::stop_tunnel(&storage, &tunnels_svc, &id_str).await;
                        tun.is_running = false;
                    }

                    // 保存持久化
                    let _ = storage.tunnels().save(&tun).await;

                    events.dispatch(&TunnelStateChangedEvent {
                        tunnel_id: id_str.clone(),
                        is_running: tun.is_running,
                    });

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = w_weak.upgrade() {
                            if let Some(msg) = warn_msg {
                                let wb = w.global::<crate::generated::WindowBridge>();
                                let toast_id = format!("toast-{}", uuid::Uuid::new_v4());
                                let toast = crate::generated::ToastItemData {
                                    id: toast_id.clone().into(),
                                    title: "启动异常".into(),
                                    message: msg.into(),
                                    level: "warning".into(),
                                    position: wb.get_toast_position(),
                                    duration_ms: 3500,
                                    closable: true,
                                };
                                let cur = wb.get_toasts();
                                let mut all: Vec<crate::generated::ToastItemData> =
                                    (0..cur.row_count()).filter_map(|i| cur.row_data(i)).collect();
                                all.push(toast);
                                wb.set_toasts(to_model_rc(all));

                                let w_weak_t = w_weak.clone();
                                let tid: slint::SharedString = toast_id.into();
                                slint::Timer::single_shot(std::time::Duration::from_millis(3500), move || {
                                    if let Some(w) = w_weak_t.upgrade() {
                                        let wb = w.global::<crate::generated::WindowBridge>();
                                        let cur = wb.get_toasts();
                                        let remaining: Vec<crate::generated::ToastItemData> =
                                            cur.iter().filter(|t| t.id != tid).collect();
                                        wb.set_toasts(to_model_rc(remaining));
                                    }
                                });
                            }
                            let tb = w.global::<TunnelsBridge>();
                            let active_id = tb.get_active_tunnel_id().to_string();
                            let form_id = tb.get_form_id().to_string();
                            if active_id == id_str || form_id == id_str {
                                tb.set_form_enabled(tun.enabled);
                                tb.set_form_is_running(tun.is_running);
                            }
                            tb.invoke_sync_host_tunnels();
                        }
                    });
                }
            });
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

    // -------------------------------------------------------------------------
    // 3. 开始新建规则
    // -------------------------------------------------------------------------
    // 触发场景：在隧道主页面点击“新建规则”按钮。
    // 业务逻辑：生成随机时间戳 ID，重置表单为标准 Local 端口转发默认参数，开启编辑模式。
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
                tb.set_form_hops(ModelRc::default());
                tb.set_form_proxy_proto("SOCKS5".into());
                tb.set_form_proxy_username("".into());
                tb.set_form_proxy_password("".into());
            }
        });
    }

    // -------------------------------------------------------------------------
    // 3.B 开始新建指定类型的网络规则 (端口转发 / 跳板机 / 出网代理)
    // -------------------------------------------------------------------------
    // 触发场景：在新建下拉菜单中明确选择规则类型：
    // - `"JumpHost"`: 多级跳板链路，初始化 hops 列表，SSH 命令默认 `-J`；
    // - `"ProxyServer"`: 出网代理服务，默认 SOCKS5 协议，常用端口 7890，启用远端 DNS 与伴随应用启动；
    // - 其它类型: 本地或远程端口转发模板。
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
                tb.set_form_hops(ModelRc::default());
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
                        tb.set_form_hops(ModelRc::default());
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
                        tb.set_form_hops(ModelRc::default());
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

    // -------------------------------------------------------------------------
    // 4. 取消新建规则
    // -------------------------------------------------------------------------
    // 触发场景：在新建表单中点击“取消”按钮。
    // 业务逻辑：退出新建与编辑状态，异步从仓储查询首条规则恢复选中，若无记录则置空。
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_cancel_create(move || {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                tb.set_is_create_mode(false);
                tb.set_is_editing(false);
                let storage = ctx.core_state.storage();
                let w_weak = w_handle.clone();
                crate::async_util::spawn_async(async move {
                    let all = storage.tunnels().list_all().await.unwrap_or_default();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = w_weak.upgrade() {
                            let tb = w.global::<TunnelsBridge>();
                            if let Some(first) = all.first() {
                                tb.set_active_tunnel_id(first.id.clone().into());
                                load_tunnel_into_bridge(&tb, first);
                            } else {
                                tb.set_active_tunnel_id("".into());
                            }
                        }
                    });
                });
            }
        });
    }

    // -------------------------------------------------------------------------
    // 5. 开始编辑规则
    // -------------------------------------------------------------------------
    // 触发场景：在规则详情面板中点击“编辑”按钮。
    // 业务逻辑：置 `is_editing` 为 true，解锁前端输入控件。
    {
        let w_handle = window.as_weak();
        tb.on_start_edit(move || {
            if let Some(w) = w_handle.upgrade() {
                w.global::<TunnelsBridge>().set_is_editing(true);
            }
        });
    }

    // -------------------------------------------------------------------------
    // 6. 取消编辑规则
    // -------------------------------------------------------------------------
    // 触发场景：在编辑状态下点击“取消修改”。
    // 业务逻辑：关闭可编辑态，从存储层重新拉取当前选中 ID 的最新持久化记录，还原被修改的表单内容。
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_cancel_edit(move || {
            if let Some(w) = w_handle.upgrade() {
                let tb = w.global::<TunnelsBridge>();
                tb.set_is_editing(false);
                let id = tb.get_active_tunnel_id().to_string();
                let storage = ctx.core_state.storage();
                let w_weak = w_handle.clone();
                crate::async_util::spawn_async(async move {
                    if let Ok(Some(tun)) = storage.tunnels().get_by_id(&id).await {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = w_weak.upgrade() {
                                let tb = w.global::<TunnelsBridge>();
                                load_tunnel_into_bridge(&tb, &tun);
                            }
                        });
                    }
                });
            }
        });
    }

    // -------------------------------------------------------------------------
    // 7. 保存规则 (新建或修改 - 严格遵循安全热重载生命周期)
    // -------------------------------------------------------------------------
    // 处理流程：
    // 1. 字段非空与合法性校验（规则名称、端口范围）；
    // 2. 派发前置审查事件 `TunnelBeforeSaveEvent`，允许插件或内置审计拦截高危修改；
    // 3. 【核心要求：修改时先关闭旧通道】若已有通道处于运行状态，首先调用 `stop_tunnel` 释放物理端口；
    // 4. 组装新实体记录并持久化保存到仓储中，派发 `TunnelSavedEvent`；
    // 5. 【核心要求：按新配置重新拉取/拉起】依据用户设定的开关与运行策略（FollowApp / FollowTerminal）决定是否即刻拉起底层通道；
    // 6. UI 状态全面水合（主页面列表与右侧专属抽屉同步更新）。
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

                let jump_host_ids: Vec<String> = jump_hops.iter().map(|h| h.host_id.clone()).collect();
                let before_save = TunnelBeforeSaveEvent::new(
                    &id_str,
                    &name_str,
                    parsed_type.as_str(),
                    if l_bind.trim().is_empty() { "127.0.0.1" } else { &l_bind },
                    l_port,
                    &final_remote_host,
                    r_port,
                    jump_host_ids,
                );
                ctx.core_state.events().dispatch(&before_save);
                if before_save.is_aborted() {
                    ctx.notify_warning("配置拦截", before_save.abort_reason().unwrap_or_else(|| "规则前置校验未通过".to_string()));
                    return;
                }

                let connected_hosts: std::collections::HashSet<String> = ctx
                    .pane_groups
                    .borrow()
                    .iter()
                    .flat_map(|g| g.tabs.iter())
                    .map(|t| t.host_id.clone())
                    .collect();

                let storage = ctx.core_state.storage();
                let tunnels_svc = ctx.core_state.tunnels();
                let events = ctx.core_state.events().clone();
                let notif = ctx.notifications.clone();
                let cat = ctx.tunnel_filter_category.borrow().clone();
                let query = ctx.tunnel_search_query.borrow().clone();
                let w_weak = w_handle.clone();

                crate::async_util::spawn_async(async move {
                    // 检查旧配置是否存在，且是否正在运行
                    let old_record = storage.tunnels().get_by_id(&id_str).await.ok().flatten();
                    let is_new = old_record.is_none();

                    // 【核心要求：修改时先关闭旧通道】
                    if let Some(ref old_tun) = old_record {
                        if old_tun.is_running {
                            TunnelDaemonService::stop_tunnel(&storage, &tunnels_svc, &old_tun.id).await;
                            events.dispatch(&TunnelStateChangedEvent {
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

                    if let Ok(()) = storage.tunnels().save(&record).await {
                        events.dispatch(&TunnelSavedEvent {
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
                                    actually_started = TunnelDaemonService::try_start_tunnel(&storage, &tunnels_svc, &record).await;
                                    if actually_started {
                                        record.is_running = true;
                                    }
                                }
                                TunnelRunMode::FollowTerminal => {
                                    let is_host_connected = record.ssh_host_id.as_ref()
                                        .map(|hid| connected_hosts.contains(hid))
                                        .unwrap_or(false);
                                    if is_host_connected {
                                        actually_started = TunnelDaemonService::try_start_tunnel(&storage, &tunnels_svc, &record).await;
                                        if actually_started {
                                            record.is_running = true;
                                        }
                                    }
                                }
                            }
                        }

                        if record.is_running {
                            events.dispatch(&TunnelStateChangedEvent {
                                tunnel_id: id_str.clone(),
                                is_running: true,
                            });
                        }

                        if record.enabled {
                            if actually_started {
                                notif.success("保存并生效", format!("'{}' 配置已更新，新通道已建立监听", record.name));
                            } else if record.run_mode == TunnelRunMode::FollowTerminal {
                                notif.success("保存成功", format!("'{}' 配置已更新并处于待命状态，将在打开终端时自动激活", record.name));
                            } else {
                                notif.warning("配置已保存", format!("'{}' 端口 {}:{} 建立监听失败，请检查端口占用", record.name, record.local_bind, record.local_port));
                            }
                        } else {
                            notif.info("保存成功", format!("'{}' 规则已保存（处于停用关闭状态）", record.name));
                        }

                        let record_clone = record.clone();
                        let w_weak_invoke = w_weak.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = w_weak_invoke.upgrade() {
                                let tb = w.global::<TunnelsBridge>();
                                tb.set_is_create_mode(false);
                                tb.set_is_editing(false);
                                tb.set_is_create_host_tunnel_modal_open(false);
                                tb.set_active_tunnel_id(id_str.into());
                                load_tunnel_into_bridge(&tb, &record_clone);
                            }
                        });
                        sync_ui_tunnels_async(w_weak.clone(), storage.clone(), cat, query);
                        sync_ui_host_tunnels_async(w_weak, storage);
                    }
                });
            }
        });
    }

    // -------------------------------------------------------------------------
    // 8. 删除规则
    // -------------------------------------------------------------------------
    // 业务流程与安全防线：
    // 1. 查询当前规则运行状态，派发前置审查 `TunnelBeforeDeleteEvent`；
    // 2. 若规则处于运行中，坚决拦截删除操作，防止意外切断正在通信的底层连接；
    // 3. 执行仓储持久化删除，分发 `TunnelDeletedEvent`；
    // 4. 重查剩余规则，焦点自动回退至第一项并重新装载表单。
    {
        let ctx = ctx.clone();
        let w_handle = window.as_weak();
        tb.on_delete_tunnel(move |id| {
            let id_str = id.to_string();
            let storage = ctx.core_state.storage();
            let events = ctx.core_state.events().clone();
            let notif = ctx.notifications.clone();
            let cat = ctx.tunnel_filter_category.borrow().clone();
            let query = ctx.tunnel_search_query.borrow().clone();
            let w_weak = w_handle.clone();

            crate::async_util::spawn_async(async move {
                // 删除前审查守卫 (运行中的规则禁止误删)
                let is_running = storage.tunnels().get_by_id(&id_str)
                    .await.ok().flatten().map(|t| t.is_running).unwrap_or(false);
                let before_del = TunnelBeforeDeleteEvent::new(&id_str, is_running);
                events.dispatch(&before_del);
                if before_del.is_aborted() {
                    notif.warning("删除拦截", before_del.abort_reason().unwrap_or_else(|| "运行中的规则禁止删除".to_string()));
                    return;
                }

                if let Ok(true) = storage.tunnels().delete(&id_str).await {
                    events.dispatch(&TunnelDeletedEvent {
                        tunnel_id: id_str,
                    });
                    notif.success("删除成功", "已从网络配置库中移除该规则");

                    let all = storage.tunnels().list_all().await.unwrap_or_default();
                    let w_weak_invoke = w_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = w_weak_invoke.upgrade() {
                            let tb = w.global::<TunnelsBridge>();
                            if let Some(first) = all.first() {
                                tb.set_active_tunnel_id(first.id.clone().into());
                                load_tunnel_into_bridge(&tb, first);
                            } else {
                                tb.set_active_tunnel_id("".into());
                            }
                        }
                    });
                    sync_ui_tunnels_async(w_weak.clone(), storage.clone(), cat, query);
                    sync_ui_host_tunnels_async(w_weak, storage);
                }
            });
        });
    }

    // -------------------------------------------------------------------------
    // 9. 复制原生 OpenSSH 命令
    // -------------------------------------------------------------------------
    // 触发场景：在详情面板中点击“复制 SSH 命令”。
    // 业务逻辑：调用 `generate_ssh_command` 将规则转换成 OpenSSH CLI 命令行参数并写入系统剪贴板。
    {
        let ctx = ctx.clone();
        tb.on_copy_ssh_command(move |id| {
            let id_str = id.to_string();
            let storage = ctx.core_state.storage();
            let notif = ctx.notifications.clone();
            crate::async_util::spawn_async(async move {
                if let Ok(Some(tun)) = storage.tunnels().get_by_id(&id_str).await {
                    let cmd = tun.generate_ssh_command(&tun.ssh_host_name, "root");
                    if let Ok(mut clip) = arboard::Clipboard::new() {
                        let _ = clip.set_text(&cmd);
                        notif.success("复制成功", format!("已将 '{}' 的 SSH 命令复制到剪贴板", tun.name));
                    }
                }
            });
        });
    }

    // -------------------------------------------------------------------------
    // 10. 过滤检索与分类切换
    // -------------------------------------------------------------------------
    // 触发场景：在搜索框中键入关键词或切换分类 Tab（全部/端口转发/跳板机/代理）。
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

    // -------------------------------------------------------------------------
    // 11. 复制自定义文本
    // -------------------------------------------------------------------------
    // 触发场景：点击表单中的 IP:Port、命令预览等文本直接复制到剪贴板。
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

    // -------------------------------------------------------------------------
    // 12. 跳板链路节点操作回调 (添加 / 删除 / 向上移 / 向下移 / 启闭切换)
    // -------------------------------------------------------------------------
    // 业务流转：对多级 Jump Host 节点做有序排列，每次改动后实时重算并更新预览 `-J host1,host2...`。
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
                tb.set_form_hops(to_model_rc(hops));
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
                    tb.set_form_hops(to_model_rc(hops));
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
                    tb.set_form_hops(to_model_rc(hops));
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
                    tb.set_form_hops(to_model_rc(hops));
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
                    tb.set_form_hops(to_model_rc(hops));
                }
            }
        });
    }

    // -------------------------------------------------------------------------
    // 13. 初始化同步当前主机专属隧道
    // -------------------------------------------------------------------------
    sync_ui_host_tunnels(window, ctx);
}
