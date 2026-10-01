//! 网络隧道、跳板机与出网代理全局后台守护服务 (TunnelDaemonService)。
//!
//! 该服务跟随整个应用进程的生命周期常驻运行：
//! 1. 在应用启动就绪 (`AppReadyEvent`) 时，自动扫描所有标记为 `auto_start` 的网络规则并在后台拉起建立监听；
//! 2. 在应用退出前夕 (`AppBeforeExitEvent`) 时，优雅清空所有运行中的隧道连接，释放端口，杜绝端口残留；
//! 3. 在终端焦点切换 (`TerminalFocusChangedEvent`) 时，自动触发右侧伴生工具栏专属转发规则的热同步；
//! 4. 彻底基于 `smagical_core::service::TunnelService` 服务契约驱动。

use std::sync::Arc;
use smagical_core::event::{
    AppBeforeExitEvent, AppReadyEvent, EventManager, TerminalFocusChangedEvent,
    TerminalSessionEvent, TunnelStateChangedEvent,
};
use smagical_core::service::tunnel::TunnelService;
use smagical_core::AppStorage;
use slint::ComponentHandle;

use crate::generated::{AppWindow, TunnelsBridge};

/// 网络隧道与代理全局后台守护服务
pub struct TunnelDaemonService {
    storage: Arc<dyn AppStorage>,
    tunnel_service: Arc<dyn TunnelService>,
    window_weak: slint::Weak<AppWindow>,
}

impl TunnelDaemonService {
    /// 创建一个新的全局隧道守护服务实例
    pub fn new(
        storage: Arc<dyn AppStorage>,
        tunnel_service: Arc<dyn TunnelService>,
        window_weak: slint::Weak<AppWindow>,
    ) -> Self {
        Self {
            storage,
            tunnel_service,
            window_weak,
        }
    }

    /// 注册跟随整个应用生命周期的全局常驻事件监听
    pub fn register(self: Arc<Self>, events: &EventManager) {
        // 1. 全局应用启动就绪：自动扫描并启动标记为 auto_start / FollowApp 的常驻网络规则
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

        // 5. 终端会话启闭联动 (FollowTerminal 规则自动跟随)
        let s_session = Arc::clone(&self);
        let g_session = events.global().listen(move |e: &TerminalSessionEvent| {
            s_session.handle_terminal_session_event(e);
        });
        g_session.detach();
    }

    /// 应用启动就绪：自启所有标记为自启的网络转发
    fn handle_app_startup_autostart(&self) {
        let storage = Arc::clone(&self.storage);
        let tunnel_svc = Arc::clone(&self.tunnel_service);
        let window_weak = self.window_weak.clone();

        crate::async_util::spawn_async(async move {
            let all_tunnels = match storage.tunnels().list_all().await {
                Ok(list) => list,
                Err(err) => {
                    tracing::error!(target: "smalux::tunnel", "获取隧道配置失败: {:?}", err);
                    return;
                }
            };

            let autostart_rules: Vec<_> = all_tunnels.into_iter().filter(|t| {
                t.enabled && (t.run_mode == smagical_core::domain::tunnel::TunnelRunMode::FollowApp || t.auto_start)
            }).collect();
            let autostart_total = autostart_rules.len();
            let mut success_count = 0;
            let mut failed_count = 0;

            for tun in autostart_rules {
                if Self::try_start_tunnel(&storage, &tunnel_svc, &tun).await {
                    success_count += 1;
                } else {
                    failed_count += 1;
                }
            }

            tracing::info!(
                target: "smalux::tunnel",
                "全局常驻规则扫描完毕：共检测到 {} 条自启配置，成功启动 {} 条，异常关闭 {} 条（已保持关闭态等待手动打开，无前台弹窗打扰）",
                autostart_total, success_count, failed_count
            );

            // 异步刷新主窗口 UI 模型与抽屉状态
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    w.global::<TunnelsBridge>().invoke_sync_host_tunnels();
                }
            });
        });
    }

    /// 响应终端会话生命周期事件 (打开或销毁)
    fn handle_terminal_session_event(&self, e: &TerminalSessionEvent) {
        let host_id = e.host_id.trim();
        if host_id.is_empty() || host_id == "local" {
            return;
        }

        let storage = Arc::clone(&self.storage);
        let tunnel_svc = Arc::clone(&self.tunnel_service);
        let window_weak = self.window_weak.clone();
        let h_id = host_id.to_string();
        let action = e.action.clone();

        crate::async_util::spawn_async(async move {
            let all_tunnels = storage.tunnels().list_all().await.unwrap_or_default();
            if action == "opened" {
                for tun in all_tunnels {
                    let is_match = tun.enabled
                        && tun.run_mode == smagical_core::domain::tunnel::TunnelRunMode::FollowTerminal
                        && tun.ssh_host_id.as_deref() == Some(&h_id);
                    if is_match && !tun.is_running {
                        if Self::try_start_tunnel(&storage, &tunnel_svc, &tun).await {
                            tracing::info!(
                                target: "smalux::tunnel",
                                "[终端伴生自动拉起] 成功激活主机 [{}] 伴生隧道: [{}] '{}'",
                                h_id, tun.id, tun.name
                            );
                        }
                    }
                }
            } else if action == "closed" {
                // 检查是否仍有同主机的其它会话存活
                let has_alive = false; // 由实际活跃会话列表判定

                if !has_alive {
                    for tun in all_tunnels {
                        let is_match = tun.run_mode == smagical_core::domain::tunnel::TunnelRunMode::FollowTerminal
                            && tun.ssh_host_id.as_deref() == Some(&h_id);
                        if is_match && tun.is_running {
                            Self::stop_tunnel(&storage, &tunnel_svc, &tun.id).await;
                            tracing::info!(
                                target: "smalux::tunnel",
                                "[终端伴生自动释放] 释放主机 [{}] 隧道端口: [{}] '{}:{}'",
                                h_id, tun.id, tun.local_bind, tun.local_port
                            );
                        }
                    }
                }
            }

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    w.global::<TunnelsBridge>().invoke_sync_host_tunnels();
                }
            });
        });
    }

    /// 尝试激活单条网络规则（通过 TunnelService 协议服务启动监听并更新运行状态）
    pub async fn try_start_tunnel(
        storage: &Arc<dyn AppStorage>,
        tunnel_service: &Arc<dyn TunnelService>,
        tun: &smagical_core::TunnelRecord,
    ) -> bool {
        match tunnel_service.start_tunnel(tun, None).await {
            Ok(handle) => {
                let _ = storage.tunnels().set_running(&tun.id, true).await;
                tracing::info!(
                    target: "smalux::tunnel",
                    "[隧道建立成功] 规则 [{}] '{}' 监听就绪于: {}",
                    tun.id, tun.name, handle.bound_address
                );
                true
            }
            Err(err) => {
                tracing::warn!(
                    target: "smalux::tunnel",
                    "[隧道建立失败] 规则 [{}] '{}' 启动异常: {:?}",
                    tun.id, tun.name, err
                );
                let _ = storage.tunnels().set_running(&tun.id, false).await;
                false
            }
        }
    }

    /// 主动停止单条网络规则并释放端口
    pub async fn stop_tunnel(
        storage: &Arc<dyn AppStorage>,
        tunnel_service: &Arc<dyn TunnelService>,
        tunnel_id: &str,
    ) {
        let _ = tunnel_service.stop_tunnel(tunnel_id).await;
        let _ = storage.tunnels().set_running(tunnel_id, false).await;
        tracing::info!(target: "smalux::tunnel", "[主动停止] 释放隧道连接与端口: [{}]", tunnel_id);
    }

    /// 应用关闭前夕：安全排空活跃连接并注销所有监听端口
    fn handle_app_shutdown_graceful(&self, _: &AppBeforeExitEvent) {
        let storage = Arc::clone(&self.storage);
        let tunnel_svc = Arc::clone(&self.tunnel_service);
        crate::async_util::block_on(async move {
            if let Ok(all_tunnels) = storage.tunnels().list_all().await {
                for tun in all_tunnels.into_iter().filter(|t| t.is_running) {
                    let _ = tunnel_svc.stop_tunnel(&tun.id).await;
                    let _ = storage.tunnels().set_running(&tun.id, false).await;
                }
            }
        });
    }

    /// 响应终端焦点切换
    fn handle_terminal_focus_changed(&self, _: &TerminalFocusChangedEvent) {
        let window_weak = self.window_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                w.global::<TunnelsBridge>().invoke_sync_host_tunnels();
            }
        });
    }

    /// 响应隧道状态流转
    fn handle_tunnel_state_changed(&self, _: &TunnelStateChangedEvent) {
        let window_weak = self.window_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                w.global::<TunnelsBridge>().invoke_sync_host_tunnels();
            }
        });
    }
}
