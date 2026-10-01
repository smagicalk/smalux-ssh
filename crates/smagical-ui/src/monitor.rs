//! 真实 Linux 系统运维性能监控指标探针引擎 (UI 适配层)。
//!
//! 底层核心协议与无 UI 指标采样已沉降至 `smagical_ssh::monitor`。
//! 本模块负责将 `LinuxSystemMetrics` 桥接映射至 Slint 视图模型 `SystemMetricsData`。

pub use smagical_ssh::monitor::*;
use crate::generated::SystemMetricsData;

/// 领域指标向 Slint UI 视图模型转换扩展特征 (Trait)
pub trait IntoSlintMetrics {
    /// 将底层纯 Rust 领域指标转换为 Slint UI 视图数据结构
    fn into_slint(self) -> SystemMetricsData;
}

impl IntoSlintMetrics for LinuxSystemMetrics {
    fn into_slint(self) -> SystemMetricsData {
        SystemMetricsData {
            host_name: self.host_name.into(),
            host_ip: self.host_ip.into(),
            os_name: self.os_name.into(),
            virt_type: self.virt_type.into(),
            uptime: self.uptime.into(),
            load_avg: self.load_avg.into(),
            fd_usage: self.fd_usage.into(),
            tasks_threads: self.tasks_threads.into(),
            active_users: self.active_users,
            cpu_usage: self.cpu_usage,
            cpu_cores: self.cpu_cores,
            cpu_freq: self.cpu_freq.into(),
            cpu_temp: self.cpu_temp.into(),
            cpu_user_pct: self.cpu_user_pct.into(),
            cpu_sys_pct: self.cpu_sys_pct.into(),
            cpu_iowait_pct: self.cpu_iowait_pct.into(),
            cpu_steal_pct: self.cpu_steal_pct.into(),
            ram_usage: self.ram_usage,
            ram_used: self.ram_used.into(),
            ram_total: self.ram_total.into(),
            ram_available: self.ram_available.into(),
            ram_cached: self.ram_cached.into(),
            ram_free: self.ram_free.into(),
            swap_enabled: self.swap_enabled,
            swap_usage: self.swap_usage,
            swap_used: self.swap_used.into(),
            swap_total: self.swap_total.into(),
            swap_free: self.swap_free.into(),
            swap_status_text: self.swap_status_text.into(),
            net_interface: self.net_interface.into(),
            net_rx_rate: self.net_rx_rate.into(),
            net_tx_rate: self.net_tx_rate.into(),
            net_total_rx: self.net_total_rx.into(),
            net_total_tx: self.net_total_tx.into(),
            tcp_inuse: self.tcp_inuse,
            tcp_tw: self.tcp_tw,
            udp_inuse: self.udp_inuse,
            net_drops: self.net_drops,
            net_errs: self.net_errs,
            ping_ms: self.ping_ms,
            probe_latency_ms: self.probe_latency_ms,
            last_update_time: self.last_update_time.into(),
            poll_interval: self.poll_interval.into(),
            cpu_sparkline_area: self.cpu_sparkline_area.into(),
            cpu_sparkline_line: self.cpu_sparkline_line.into(),
            net_rx_sparkline_area: self.net_rx_sparkline_area.into(),
            net_rx_sparkline_line: self.net_rx_sparkline_line.into(),
            net_tx_sparkline_area: self.net_tx_sparkline_area.into(),
            net_tx_sparkline_line: self.net_tx_sparkline_line.into(),
            memory_usage: self.memory_usage,
            disk_usage: self.disk_usage,
            uptime_days: self.uptime_days,
            process_count: self.process_count,
        }
    }
}
