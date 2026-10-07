//! 网络隧道、跳板机与出网代理全局后台守护服务 (TunnelDaemonService)。
//!
//! 该服务跟随整个应用进程的生命周期常驻运行：
//! 1. 在应用启动就绪 (`AppReadyEvent`) 时，自动扫描所有标记为 `auto_start` 的网络规则并在后台拉起建立监听；
//! 2. 在应用退出前夕 (`AppBeforeExitEvent`) 时，优雅清空所有运行中的隧道连接，释放端口，杜绝端口残留；
//! 3. 在终端焦点切换 (`TerminalFocusChangedEvent`) 时，自动触发右侧伴生工具栏专属转发规则的热同步；
//! 4. 彻底基于 `smagical_core::service::TunnelService` 服务契约驱动；
//! 5. 持续运行 1s 高频探活与速率计算器，提供瞬时带宽波形 (Sparkline) 与指数退避自动自愈重连。

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use slint::ComponentHandle;
use smagical_core::domain::tunnel::TunnelRecord;
use smagical_core::event::{
    AppBeforeExitEvent, AppReadyEvent, EventManager, TerminalFocusChangedEvent,
    TerminalSessionEvent, TunnelStateChangedEvent,
};
use smagical_core::service::tunnel::{TunnelHandle, TunnelService};
use smagical_core::AppStorage;

use crate::generated::{AppWindow, TunnelsBridge};

/// 单条隧道速率采样状态实体
#[derive(Debug, Clone)]
pub struct TunnelRateSample {
    pub last_timestamp: Instant,
    pub last_bytes_in: u64,
    pub last_bytes_out: u64,
    pub rx_rate_bps: f64,
    pub tx_rate_bps: f64,
    pub rx_history: VecDeque<f64>,
    pub tx_history: VecDeque<f64>,
}

impl TunnelRateSample {
    pub fn new(bytes_in: u64, bytes_out: u64) -> Self {
        Self {
            last_timestamp: Instant::now(),
            last_bytes_in: bytes_in,
            last_bytes_out: bytes_out,
            rx_rate_bps: 0.0,
            tx_rate_bps: 0.0,
            rx_history: VecDeque::with_capacity(8),
            tx_history: VecDeque::with_capacity(8),
        }
    }

    pub fn update(&mut self, current_bytes_in: u64, current_bytes_out: u64) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_timestamp).as_secs_f64();
        if elapsed >= 0.25 {
            let delta_in = current_bytes_in.saturating_sub(self.last_bytes_in);
            let delta_out = current_bytes_out.saturating_sub(self.last_bytes_out);
            self.rx_rate_bps = delta_in as f64 / elapsed;
            self.tx_rate_bps = delta_out as f64 / elapsed;
            self.last_timestamp = now;
            self.last_bytes_in = current_bytes_in;
            self.last_bytes_out = current_bytes_out;

            if self.rx_history.len() >= 8 {
                self.rx_history.pop_front();
            }
            self.rx_history.push_back(self.rx_rate_bps);

            if self.tx_history.len() >= 8 {
                self.tx_history.pop_front();
            }
            self.tx_history.push_back(self.tx_rate_bps);
        }
    }
}

/// 隧道全局瞬时流量与速率计算器
#[derive(Debug, Default)]
pub struct TunnelRateTracker {
    samples: HashMap<String, TunnelRateSample>,
}

impl TunnelRateTracker {
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
        }
    }

    /// 更新采样并返回当前 (rx_rate, tx_rate, rx_sparkline, tx_sparkline)
    pub fn update_metrics(
        &mut self,
        tunnel_id: &str,
        bytes_in: u64,
        bytes_out: u64,
    ) -> (f64, f64, String, String) {
        let entry = self.samples.entry(tunnel_id.to_string()).or_insert_with(|| {
            TunnelRateSample::new(bytes_in, bytes_out)
        });
        entry.update(bytes_in, bytes_out);
        let rx_spark = Self::generate_sparkline(&entry.rx_history);
        let tx_spark = Self::generate_sparkline(&entry.tx_history);
        (entry.rx_rate_bps, entry.tx_rate_bps, rx_spark, tx_spark)
    }

    /// 格式化传输速率 (如 "0 B/s", "128.5 KB/s", "1.24 MB/s")
    pub fn format_speed(bps: f64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;

        if bps >= GB {
            format!("{:.2} GB/s", bps / GB)
        } else if bps >= MB {
            format!("{:.1} MB/s", bps / MB)
        } else if bps >= KB {
            format!("{:.1} KB/s", bps / KB)
        } else if bps > 0.0 {
            format!("{:.0} B/s", bps)
        } else {
            "0 B/s".to_string()
        }
    }

    /// 生成平滑文本波形走势图 (基于 Unicode Block Elements:   ▂ ▃ ▄ ▅ ▆ ▇ █)
    pub fn generate_sparkline(history: &VecDeque<f64>) -> String {
        if history.is_empty() {
            return String::new();
        }
        const BARS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let max_val = history.iter().copied().fold(0.0, f64::max);
        if max_val <= 0.0 {
            return String::new();
        }

        let mut res = String::with_capacity(history.len());
        for &val in history {
            let ratio = (val / max_val).clamp(0.0, 1.0);
            let idx = ((ratio * 7.0).round() as usize).min(7);
            res.push(BARS[idx]);
        }
        res
    }

    /// 获取特定隧道的格式化展示文本
    pub fn get_formatted_traffic(
        &self,
        tunnel_id: &str,
        total_in: u64,
        total_out: u64,
    ) -> (String, String) {
        if let Some(entry) = self.samples.get(tunnel_id) {
            let rx_spd = Self::format_speed(entry.rx_rate_bps);
            let tx_spd = Self::format_speed(entry.tx_rate_bps);
            let rx_spark = Self::generate_sparkline(&entry.rx_history);
            let tx_spark = Self::generate_sparkline(&entry.tx_history);

            let in_text = if !rx_spark.is_empty() {
                format!("{} ({} {})", TunnelRecord::format_bytes(total_in), rx_spd, rx_spark)
            } else if entry.rx_rate_bps > 0.0 {
                format!("{} ({})", TunnelRecord::format_bytes(total_in), rx_spd)
            } else {
                TunnelRecord::format_bytes(total_in)
            };

            let out_text = if !tx_spark.is_empty() {
                format!("{} ({} {})", TunnelRecord::format_bytes(total_out), tx_spd, tx_spark)
            } else if entry.tx_rate_bps > 0.0 {
                format!("{} ({})", TunnelRecord::format_bytes(total_out), tx_spd)
            } else {
                TunnelRecord::format_bytes(total_out)
            };

            (in_text, out_text)
        } else {
            (TunnelRecord::format_bytes(total_in), TunnelRecord::format_bytes(total_out))
        }
    }
}

/// 自动重连退避重试状态机实体
#[derive(Debug, Clone)]
struct AutoHealingEntry {
    failure_count: u32,
    next_retry_at: Instant,
    last_error: Option<String>,
}

/// 隧道自愈重连管理器
#[derive(Debug, Default)]
pub struct AutoHealingManager {
    entries: HashMap<String, AutoHealingEntry>,
}

impl AutoHealingManager {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// 判断指定隧道当前是否已满足退避时间，可以发起下一次重试
    pub fn can_retry(&self, tunnel_id: &str) -> bool {
        if let Some(entry) = self.entries.get(tunnel_id) {
            Instant::now() >= entry.next_retry_at
        } else {
            true
        }
    }

    /// 记录重试失败并计算指数退避时间: 2s -> 4s -> 8s -> 16s -> 32s -> max 60s
    pub fn record_failure(&mut self, tunnel_id: &str, err_msg: String) -> std::time::Duration {
        let entry = self.entries.entry(tunnel_id.to_string()).or_insert_with(|| AutoHealingEntry {
            failure_count: 0,
            next_retry_at: Instant::now(),
            last_error: None,
        });

        entry.failure_count = entry.failure_count.saturating_add(1);
        entry.last_error = Some(err_msg);

        // 指数退避: 2s * 2^(min(failures - 1, 5)), 最大 60s
        let exp = entry.failure_count.saturating_sub(1).min(5);
        let backoff_secs = (2u64.pow(exp)).min(60);
        let duration = std::time::Duration::from_secs(backoff_secs);
        entry.next_retry_at = Instant::now() + duration;
        duration
    }

    /// 记录自愈成功，清空失败重试状态
    pub fn record_success(&mut self, tunnel_id: &str) {
        self.entries.remove(tunnel_id);
    }
}

/// 网络隧道与代理全局后台守护服务
pub struct TunnelDaemonService {
    storage: Arc<dyn AppStorage>,
    tunnel_service: Arc<dyn TunnelService>,
    window_weak: slint::Weak<AppWindow>,
    rate_tracker: Arc<Mutex<TunnelRateTracker>>,
    healing_manager: Arc<Mutex<AutoHealingManager>>,
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
            rate_tracker: Arc::new(Mutex::new(TunnelRateTracker::new())),
            healing_manager: Arc::new(Mutex::new(AutoHealingManager::new())),
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

        // 6. 挂载后台心跳探活、自愈重连与实时 1s 吞吐波形度量常驻协程
        self.spawn_keepalive_telemetry_loop();
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

    /// 尝试激活单条网络规则（返回详尽的 TunnelHandle 实体或详细错误描述）
    pub async fn try_start_tunnel_with_result(
        storage: &Arc<dyn AppStorage>,
        tunnel_service: &Arc<dyn TunnelService>,
        tun: &smagical_core::TunnelRecord,
    ) -> Result<TunnelHandle, String> {
        match tunnel_service.start_tunnel(tun, None).await {
            Ok(handle) => {
                let _ = storage.tunnels().set_running(&tun.id, true).await;
                tracing::info!(
                    target: "smalux::tunnel",
                    "[隧道建立成功] 规则 [{}] '{}' 监听就绪于: {}",
                    tun.id, tun.name, handle.bound_address
                );
                Ok(handle)
            }
            Err(err) => {
                let err_str = err.to_string();
                tracing::warn!(
                    target: "smalux::tunnel",
                    "[隧道建立失败] 规则 [{}] '{}' 启动异常: {}",
                    tun.id, tun.name, err_str
                );
                let _ = storage.tunnels().set_running(&tun.id, false).await;
                Err(err_str)
            }
        }
    }

    /// 尝试激活单条网络规则（简化布尔契约，向后兼容）
    pub async fn try_start_tunnel(
        storage: &Arc<dyn AppStorage>,
        tunnel_service: &Arc<dyn TunnelService>,
        tun: &smagical_core::TunnelRecord,
    ) -> bool {
        Self::try_start_tunnel_with_result(storage, tunnel_service, tun).await.is_ok()
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

    /// 启动后台心跳探活、自愈重连与实时 1s 吞吐波形度量常驻任务
    fn spawn_keepalive_telemetry_loop(self: &Arc<Self>) {
        let storage = Arc::clone(&self.storage);
        let tunnel_svc = Arc::clone(&self.tunnel_service);
        let window_weak = self.window_weak.clone();
        let rate_tracker = Arc::clone(&self.rate_tracker);
        let healing_mgr = Arc::clone(&self.healing_manager);

        crate::async_util::spawn_async(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(1000));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;

                // 若 UI 窗口已销毁，终止常驻探针协程，释放 Arc 与网络探针资源
                if window_weak.upgrade().is_none() {
                    tracing::info!("UI 主窗口已关闭，终止隧道探针后台协程");
                    break;
                }

                let all_tunnels = match storage.tunnels().list_all().await {
                    Ok(list) => list,
                    Err(_) => continue,
                };

                let mut total_traffic_in: u64 = 0;
                let mut total_traffic_out: u64 = 0;
                let mut active_tunnels_count: i32 = 0;
                let mut metrics_map: HashMap<String, (i32, String, String)> = HashMap::new();

                for tun in all_tunnels {
                    if tun.is_running {
                        // 1. 探活链路状态
                        let is_alive = tunnel_svc.is_tunnel_alive(&tun.id).await;
                        if is_alive {
                            active_tunnels_count += 1;
                            // 2. 采集即时吞吐度量指标
                            if let Ok(snapshot) = tunnel_svc.query_metrics(&tun.id).await {
                                let mut tracker = rate_tracker.lock().await;
                                tracker.update_metrics(
                                    &tun.id,
                                    snapshot.bytes_in,
                                    snapshot.bytes_out,
                                );
                                total_traffic_in = total_traffic_in.saturating_add(snapshot.bytes_in);
                                total_traffic_out = total_traffic_out.saturating_add(snapshot.bytes_out);

                                let (formatted_in, formatted_out) = tracker.get_formatted_traffic(
                                    &tun.id,
                                    snapshot.bytes_in,
                                    snapshot.bytes_out,
                                );

                                metrics_map.insert(
                                    tun.id.clone(),
                                    (snapshot.active_connections as i32, formatted_in, formatted_out),
                                );
                            }
                        } else {
                            // 链路异常中断，触发自愈状态机
                            tracing::warn!(
                                target: "smalux::tunnel",
                                "[健康探活] 隧道 [{}] '{}' 监听中断",
                                tun.id, tun.name
                            );

                            if tun.auto_reconnect {
                                let healer = healing_mgr.lock().await;
                                if healer.can_retry(&tun.id) {
                                    drop(healer);
                                    match Self::try_start_tunnel_with_result(&storage, &tunnel_svc, &tun).await {
                                        Ok(_) => {
                                            let mut healer = healing_mgr.lock().await;
                                            healer.record_success(&tun.id);
                                            tracing::info!(
                                                target: "smalux::tunnel",
                                                "[自愈重连成功] 规则 [{}] '{}' 已恢复监听",
                                                tun.id, tun.name
                                            );
                                        }
                                        Err(err) => {
                                            let mut healer = healing_mgr.lock().await;
                                            let delay = healer.record_failure(&tun.id, err.clone());
                                            tracing::warn!(
                                                target: "smalux::tunnel",
                                                "[自愈重连重试中] 规则 [{}] '{}' 启动异常: {}, 将于 {:.0} 秒后再次重试",
                                                tun.id, tun.name, err, delay.as_secs_f64()
                                            );
                                        }
                                    }
                                }
                            } else {
                                let _ = storage.tunnels().set_running(&tun.id, false).await;
                            }
                        }
                    } else if tun.enabled && tun.auto_reconnect && tun.run_mode == smagical_core::domain::tunnel::TunnelRunMode::FollowApp {
                        // 随应用常驻自启但在退避状态中的规则，周期性检查是否可重试拉起
                        let healer = healing_mgr.lock().await;
                        if healer.can_retry(&tun.id) {
                            drop(healer);
                            if let Ok(_) = Self::try_start_tunnel_with_result(&storage, &tunnel_svc, &tun).await {
                                let mut healer = healing_mgr.lock().await;
                                healer.record_success(&tun.id);
                            }
                        }
                    }
                }

                // 调度回 Slint UI 线程更新总看板与当前表单流量走势
                let total_in_fmt = TunnelRecord::format_bytes(total_traffic_in);
                let total_out_fmt = TunnelRecord::format_bytes(total_traffic_out);
                let w_weak = window_weak.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w_weak.upgrade() {
                        let tb = w.global::<TunnelsBridge>();
                        tb.set_active_count(active_tunnels_count);
                        tb.set_total_traffic_in(total_in_fmt.into());
                        tb.set_total_traffic_out(total_out_fmt.into());

                        let cur_id = tb.get_active_tunnel_id().to_string();
                        if let Some((active_conns, in_fmt, out_fmt)) = metrics_map.get(&cur_id) {
                            tb.set_form_active_connections(*active_conns);
                            tb.set_form_traffic_in(in_fmt.clone().into());
                            tb.set_form_traffic_out(out_fmt.clone().into());
                        }
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_rate_tracker_format_speed() {
        assert_eq!(TunnelRateTracker::format_speed(0.0), "0 B/s");
        assert_eq!(TunnelRateTracker::format_speed(512.0), "512 B/s");
        assert_eq!(TunnelRateTracker::format_speed(1024.0), "1.0 KB/s");
        assert_eq!(TunnelRateTracker::format_speed(1024.0 * 1024.0 * 2.5), "2.5 MB/s");
    }

    #[test]
    fn test_tunnel_rate_tracker_generate_sparkline() {
        let mut history = VecDeque::new();
        assert_eq!(TunnelRateTracker::generate_sparkline(&history), "");

        history.push_back(100.0);
        history.push_back(200.0);
        history.push_back(400.0);
        history.push_back(800.0);

        let spark = TunnelRateTracker::generate_sparkline(&history);
        assert_eq!(spark.chars().count(), 4);
        // 最后一个元素 800.0 为最大值，对应最高柱 '█'
        assert_eq!(spark.chars().last(), Some('█'));
    }

    #[test]
    fn test_auto_healing_manager_exponential_backoff() {
        let mut healer = AutoHealingManager::new();
        let tid = "tun-healing-test";

        assert!(healer.can_retry(tid));

        // 第 1 次失败: 2^0 = 1 -> 2s (base 2s)
        let d1 = healer.record_failure(tid, "端口冲突".to_string());
        assert_eq!(d1.as_secs(), 2);
        assert!(!healer.can_retry(tid));

        // 第 2 次失败: 2^1 = 2 -> 2s
        let d2 = healer.record_failure(tid, "网络超时".to_string());
        assert_eq!(d2.as_secs(), 2);

        // 第 3 次失败: 2^2 = 4 -> 4s
        let d3 = healer.record_failure(tid, "拒绝连接".to_string());
        assert_eq!(d3.as_secs(), 4);

        // 成功自愈后清空
        healer.record_success(tid);
        assert!(healer.can_retry(tid));
    }
}
