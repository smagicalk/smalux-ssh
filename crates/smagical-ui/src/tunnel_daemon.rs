//! 网络隧道、跳板机与出网代理全局后台守护服务 (TunnelDaemonService)。
//!
//! 该服务跟随整个应用进程的生命周期常驻运行：
//! 1. 在应用启动就绪 (`AppReadyEvent`) 时，自动扫描所有标记为 `auto_start` 的网络规则并在后台拉起建立监听；
//! 2. 在应用退出前夕 (`AppBeforeExitEvent`) 时，优雅清空所有运行中的隧道连接，释放端口，杜绝端口残留；
//! 3. 在终端焦点切换 (`TerminalFocusChangedEvent`) 时，自动触发右侧伴生工具栏专属转发规则的热同步。

use std::sync::Arc;
use smagical_core::event::{
    AppBeforeExitEvent, AppReadyEvent, EventManager, TerminalFocusChangedEvent,
    TunnelStateChangedEvent,
};
use smagical_core::AppStorage;

use crate::generated::AppWindow;

/// 网络隧道与代理全局后台守护服务
pub struct TunnelDaemonService {
    storage: Arc<dyn AppStorage>,
    window_weak: slint::Weak<AppWindow>,
}

impl TunnelDaemonService {
    /// 创建一个新的全局隧道守护服务实例
    pub fn new(storage: Arc<dyn AppStorage>, window_weak: slint::Weak<AppWindow>) -> Self {
        Self {
            storage,
            window_weak,
        }
    }

    /// 注册跟随整个应用生命周期的全局常驻事件监听
    pub fn register(self: Arc<Self>, events: &EventManager) {
        // 1. 全局应用启动就绪：自动扫描并启动标记为 auto_start 的网络规则
        let s_ready = Arc::clone(&self);
        let g_ready = events.global().listen(move |_: &AppReadyEvent| {
            s_ready.handle_app_startup_autostart();
        });
        g_ready.detach();

        // 2. 全局应用退出前夕：安全排空活跃连接并注销所有监听端口
        let s_exit = Arc::clone(&self);
        let g_exit = events.global().listen(move |e: &AppBeforeExitEvent| {
            s_exit.handle_app_shutdown_graceful(e);
        });
        g_exit.detach();

        // 3. 终端焦点切换：驱动右侧伴生抽屉按主机过滤规则热同步
        let s_focus = Arc::clone(&self);
        let g_focus = events.global().listen(move |e: &TerminalFocusChangedEvent| {
            s_focus.handle_terminal_focus_changed(e);
        });
        g_focus.detach();

        // 4. 隧道启停状态流转：同步刷新 UI 状态与伴生工具栏
        let s_state = Arc::clone(&self);
        let g_state = events.global().listen(move |e: &TunnelStateChangedEvent| {
            s_state.handle_tunnel_state_changed(e);
        });
        g_state.detach();
    }

    /// 应用引导启动时执行自启规则扫描与可用性探测。
    /// 若某个转发有错误就自动关闭该转发，等待用户手动打开，无需向用户弹窗提示。
    fn handle_app_startup_autostart(&self) {
        let storage = Arc::clone(&self.storage);
        let window_weak = self.window_weak.clone();

        std::thread::Builder::new()
            .name("tunnel-autostart-daemon".into())
            .spawn(move || {
                tracing::info!(target: "smalux::tunnel", "应用首帧就绪，开始扫描并自启标记为 auto_start 的网络隧道与代理...");

                let all_tunnels = match storage.tunnels().list_all() {
                    Ok(list) => list,
                    Err(e) => {
                        tracing::error!(target: "smalux::tunnel", "读取网络规则配置库失败: {:?}", e);
                        return;
                    }
                };

                let autostart_rules: Vec<_> = all_tunnels.into_iter().filter(|t| t.auto_start).collect();
                let autostart_total = autostart_rules.len();
                let mut success_count = 0;
                let mut failed_count = 0;

                for tun in autostart_rules {
                    if Self::try_start_tunnel_on_boot(&storage, &tun) {
                        success_count += 1;
                    } else {
                        failed_count += 1;
                    }
                }

                tracing::info!(
                    target: "smalux::tunnel",
                    "全局自启规则扫描完毕：共检测到 {} 条自启配置，成功启动 {} 条，异常关闭 {} 条（已保持关闭态等待手动打开，无前台弹窗打扰）",
                    autostart_total, success_count, failed_count
                );

                // 异步刷新主窗口 UI 模型与抽屉状态
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = window_weak.upgrade() {
                        w.invoke_sync_host_tunnels();
                    }
                });
            })
            .ok();
    }

    /// 尝试在启动时激活单条网络规则。
    /// 若探测失败或发生配置/端口冲突错误，则静默关闭该规则并等待手动打开（不显示任何 UI 提示）。
    fn try_start_tunnel_on_boot(storage: &Arc<dyn AppStorage>, tun: &smagical_core::TunnelRecord) -> bool {
        // 基础配置与端口可用性探测
        match tun.tunnel_type {
            smagical_core::TunnelType::Local | smagical_core::TunnelType::Dynamic => {
                if tun.local_port == 0 {
                    tracing::warn!(
                        target: "smalux::tunnel",
                        "[自启失败] 规则 [{}] '{}' 本地端口为 0，静默关闭该转发，等待手动打开",
                        tun.id, tun.name
                    );
                    let _ = storage.tunnels().set_running(&tun.id, false);
                    return false;
                }

                // 探测本地监听地址与端口是否可绑定
                let bind_ip = if tun.local_bind.trim().is_empty() {
                    "127.0.0.1"
                } else {
                    tun.local_bind.trim()
                };
                match std::net::TcpListener::bind((bind_ip, tun.local_port)) {
                    Ok(listener) => {
                        // 端口探测可用，立即释放临时探测句柄以供实际隧道使用
                        drop(listener);
                    }
                    Err(err) => {
                        tracing::warn!(
                            target: "smalux::tunnel",
                            "[自启失败] 规则 [{}] '{}' 绑定本地端口 {}:{} 失败 ({:?})，静默关闭该转发，等待手动打开",
                            tun.id, tun.name, bind_ip, tun.local_port, err
                        );
                        let _ = storage.tunnels().set_running(&tun.id, false);
                        return false;
                    }
                }
            }
            smagical_core::TunnelType::Remote | smagical_core::TunnelType::ReverseDynamic => {
                if tun.remote_port == 0 {
                    tracing::warn!(
                        target: "smalux::tunnel",
                        "[自启失败] 规则 [{}] '{}' 远端监听端口为 0，静默关闭该转发，等待手动打开",
                        tun.id, tun.name
                    );
                    let _ = storage.tunnels().set_running(&tun.id, false);
                    return false;
                }
            }
            smagical_core::TunnelType::JumpHost => {
                let enabled_hops = tun.jump_chain.iter().filter(|h| h.enabled).count();
                if enabled_hops == 0 {
                    tracing::warn!(
                        target: "smalux::tunnel",
                        "[自启失败] 规则 [{}] '{}' 跳板链中无可用的启用节点，静默关闭该转发，等待手动打开",
                        tun.id, tun.name
                    );
                    let _ = storage.tunnels().set_running(&tun.id, false);
                    return false;
                }
            }
            smagical_core::TunnelType::ProxyServer => {
                if tun.local_port > 0 {
                    let bind_ip = if tun.local_bind.trim().is_empty() {
                        "127.0.0.1"
                    } else {
                        tun.local_bind.trim()
                    };
                    if let Err(err) = std::net::TcpListener::bind((bind_ip, tun.local_port)) {
                        tracing::warn!(
                            target: "smalux::tunnel",
                            "[自启失败] 代理规则 [{}] '{}' 端口 {}:{} 绑定失败 ({:?})，静默关闭该转发，等待手动打开",
                            tun.id, tun.name, bind_ip, tun.local_port, err
                        );
                        let _ = storage.tunnels().set_running(&tun.id, false);
                        return false;
                    }
                }
            }
        }

        // 探测成功，标记为运行状态
        tracing::info!(
            target: "smalux::tunnel",
            "[全局自启成功] 规则 [{}] '{}' ({}) 端口 {}:{} 已正常激活",
            tun.id, tun.name, tun.tunnel_type, tun.local_bind, tun.local_port
        );
        let _ = storage.tunnels().set_running(&tun.id, true);
        true
    }

    /// 应用退出前优雅清理所有运行中的网络规则
    fn handle_app_shutdown_graceful(&self, _e: &AppBeforeExitEvent) {
        tracing::info!(target: "smalux::tunnel", "收到全局退出事件，开始优雅关闭所有运行中的网络隧道...");

        if let Ok(tunnels) = self.storage.tunnels().list_all() {
            let running_tunnels: Vec<_> = tunnels.into_iter().filter(|t| t.is_running).collect();
            for tun in running_tunnels {
                tracing::info!(
                    target: "smalux::tunnel",
                    "[全局注销] 释放网络规则端口: [{}] '{}:{}'",
                    tun.id, tun.local_bind, tun.local_port
                );
                let _ = self.storage.tunnels().set_running(&tun.id, false);
            }
        }
    }

    /// 终端焦点切换时，通知右侧伴生工具栏更新专属隧道
    fn handle_terminal_focus_changed(&self, e: &TerminalFocusChangedEvent) {
        let h_id = e.host_id.clone().unwrap_or_default();
        let window_weak = self.window_weak.clone();

        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                w.set_active_host_id(h_id.into());
                w.invoke_sync_host_tunnels();
            }
        });
    }

    /// 隧道状态发生变更时，通知 UI 刷新
    fn handle_tunnel_state_changed(&self, e: &TunnelStateChangedEvent) {
        let tun_id = e.tunnel_id.clone();
        let is_running = e.is_running;
        let window_weak = self.window_weak.clone();

        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                let active_id = w.get_active_tunnel_id().to_string();
                if active_id == tun_id {
                    w.set_tunnel_form_is_running(is_running);
                }
                w.invoke_sync_host_tunnels();
            }
        });
    }
}


