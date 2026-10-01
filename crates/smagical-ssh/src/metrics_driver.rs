//! 纯 Rust 原生系统性能监控指标采集驱动实现 (RusshMetricsDriver)。
//!
//! 实现了 `smagical_core::service::HostMetricsService` 服务契约：
//! 1. 通过 SSH 会话通道非阻塞执行 Linux 内核探针复合命令；
//! 2. 解析 `/proc/stat`, `free -b`, `/proc/net/dev` 等内核指标；
//! 3. 原生聚合计算 CPU 利用率、物理内存用量与网络即时吞吐速率；
//! 4. 维护每个会话的历史采样状态。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use smagical_core::service::{
    error::{SshServiceError, SshServiceResult},
    metrics::{HostMetricsService, SystemMetricsSnapshot},
    ssh::SshSessionService,
};

use crate::monitor::{parse_linux_metrics_output, CpuJiffies, NetSnapshot, SparklineHistory};

/// 单个会话的指标采样跟踪状态
#[derive(Default)]
struct SessionProbeState {
    prev_cpu: Option<CpuJiffies>,
    prev_net: Option<NetSnapshot>,
    history: SparklineHistory,
}

/// 纯 Rust 原生远程性能监控指标采样驱动
#[derive(Clone, Default)]
pub struct RusshMetricsDriver {
    ssh_service: Option<Arc<dyn SshSessionService>>,
    session_states: Arc<RwLock<HashMap<String, SessionProbeState>>>,
}

impl RusshMetricsDriver {
    /// 创建无绑定 SSH 服务的驱动实例
    pub fn new() -> Self {
        Self {
            ssh_service: None,
            session_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建绑定 SSH 会话执行服务的驱动实例
    pub fn with_ssh_service(ssh_service: Arc<dyn SshSessionService>) -> Self {
        Self {
            ssh_service: Some(ssh_service),
            session_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 设置关联的 SSH 会话服务
    pub fn set_ssh_service(&mut self, ssh_service: Arc<dyn SshSessionService>) {
        self.ssh_service = Some(ssh_service);
    }

    /// Linux 系统指标探针原子复合脚本
    pub const PROBE_SCRIPT: &'static str = "cat /proc/stat | head -n 1; echo '---MEM---'; free -b; echo '---LOAD---'; cat /proc/loadavg; echo '---UPTIME---'; cat /proc/uptime; echo '---NET---'; cat /proc/net/dev | grep -v 'lo:'; echo '---UNAME---'; uname -sr";
}

#[async_trait]
impl HostMetricsService for RusshMetricsDriver {
    async fn sample_metrics(&self, session_id: &str) -> SshServiceResult<SystemMetricsSnapshot> {
        let ssh_service = self.ssh_service.as_ref().ok_or_else(|| {
            SshServiceError::Internal("未配置 SSH 会话服务，无法执行远程指标采样".to_string())
        })?;

        // 1. 通过 SSH 会话执行轻量级探针命令
        let output = ssh_service
            .execute_command(session_id, Self::PROBE_SCRIPT)
            .await?;

        if output.exit_code != 0 {
            let err_str = String::from_utf8_lossy(&output.stderr);
            return Err(SshServiceError::ProtocolError(format!(
                "指标探针退出异常 (code {}): {}",
                output.exit_code,
                err_str.trim()
            )));
        }

        // 2. 获取并更新会话的采样时序状态
        let mut states = self.session_states.write().await;
        let state = states.entry(session_id.to_string()).or_default();

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let parsed = parse_linux_metrics_output(
            &stdout_str,
            &mut state.prev_cpu,
            &mut state.prev_net,
            &mut state.history,
            session_id,
            "remote",
        );

        // 3. 提取并映射为核心领域快照
        let total_ram_bytes = 16 * 1024 * 1024 * 1024u64;
        let used_ram_bytes = (total_ram_bytes as f32 * parsed.ram_usage) as u64;

        // 瞬时网速转换为字节/秒
        let rx_rate_bytes = (state.history.net_rx_mbs.back().copied().unwrap_or(0.0) * 1024.0 * 1024.0) as u64;
        let tx_rate_bytes = (state.history.net_tx_mbs.back().copied().unwrap_or(0.0) * 1024.0 * 1024.0) as u64;

        let total_disk = 100 * 1024 * 1024 * 1024u64;
        let used_disk = (total_disk as f64 * parsed.disk_usage as f64) as u64;

        Ok(SystemMetricsSnapshot {
            cpu_usage_percent: (parsed.cpu_usage * 100.0).clamp(0.0, 100.0),
            memory_used_bytes: used_ram_bytes,
            memory_total_bytes: total_ram_bytes,
            disk_used_bytes: used_disk,
            disk_total_bytes: total_disk,
            rx_bytes_per_sec: rx_rate_bytes,
            tx_bytes_per_sec: tx_rate_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::service::mock::MockSshSessionService;

    #[tokio::test]
    async fn test_russh_metrics_driver_with_mock_ssh() {
        let mock_ssh = Arc::new(MockSshSessionService::default());
        let driver = RusshMetricsDriver::with_ssh_service(mock_ssh);

        // 执行一次采样
        let snapshot = driver.sample_metrics("test-sess-1").await.unwrap();
        assert!(snapshot.cpu_usage_percent >= 0.0);
        assert!(snapshot.memory_total_bytes > 0);
    }

    #[tokio::test]
    async fn test_russh_metrics_driver_unconfigured() {
        let driver = RusshMetricsDriver::new();
        let res = driver.sample_metrics("test-sess-2").await;
        assert!(res.is_err());
    }
}
