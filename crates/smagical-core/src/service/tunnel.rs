//! 网络隧道与端口转发契约。

use crate::domain::{CredentialRecord, TunnelRecord};
use crate::service::error::SshServiceResult;
use async_trait::async_trait;

/// 运行中网络隧道的控制句柄
#[derive(Debug, Clone)]
pub struct TunnelHandle {
    /// 隧道记录唯一 ID
    pub tunnel_id: String,
    /// 实际本地监听地址 (例如 "127.0.0.1:1080")
    pub bound_address: String,
    /// 是否处于活跃运行状态
    pub is_active: bool,
}

/// 隧道实时吞吐与连接指标快照
#[derive(Debug, Clone, Default)]
pub struct TunnelMetricsSnapshot {
    /// 当前活跃连接数
    pub active_connections: usize,
    /// 入向累计字节数
    pub bytes_in: u64,
    /// 出向累计字节数
    pub bytes_out: u64,
}

/// 网络隧道与端口转发抽象契约
#[async_trait]
pub trait TunnelService: Send + Sync {
    /// 根据配置启动一条隧道或代理规则
    async fn start_tunnel(
        &self,
        tunnel: &TunnelRecord,
        credential: Option<&CredentialRecord>,
    ) -> SshServiceResult<TunnelHandle>;

    /// 停止指定运行中的网络规则
    async fn stop_tunnel(&self, tunnel_id: &str) -> SshServiceResult<()>;

    /// 查询指定隧道的即时吞吐度量指标
    async fn query_metrics(&self, tunnel_id: &str) -> SshServiceResult<TunnelMetricsSnapshot>;

    /// 探活指定隧道的健康状态 (返回 true 表示运行正常，false 表示已断开或异常)
    async fn is_tunnel_alive(&self, tunnel_id: &str) -> bool {
        self.query_metrics(tunnel_id).await.is_ok()
    }
}

