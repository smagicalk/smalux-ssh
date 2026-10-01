//! 远程主机性能指标采集契约。

use crate::service::error::SshServiceResult;
use async_trait::async_trait;

/// 系统性能指标采样快照
#[derive(Debug, Clone, Default)]
pub struct SystemMetricsSnapshot {
    /// CPU 总利用率百分比 (0.0 ~ 100.0)
    pub cpu_usage_percent: f32,
    /// 物理内存已使用字节数
    pub memory_used_bytes: u64,
    /// 物理内存总字节数
    pub memory_total_bytes: u64,
    /// 磁盘已使用字节数
    pub disk_used_bytes: u64,
    /// 磁盘总字节数
    pub disk_total_bytes: u64,
    /// 网络下行即时速率 (字节/秒)
    pub rx_bytes_per_sec: u64,
    /// 网络上行即时速率 (字节/秒)
    pub tx_bytes_per_sec: u64,
}

/// 远程性能监控采集服务契约
#[async_trait]
pub trait HostMetricsService: Send + Sync {
    /// 采集指定远程主机的系统即时负载快照
    async fn sample_metrics(&self, session_id: &str) -> SshServiceResult<SystemMetricsSnapshot>;
}
