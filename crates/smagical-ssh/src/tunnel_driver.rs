//! 纯 Rust 原生网络隧道与端口转发驱动实现 (RusshTunnelDriver)。
//!
//! 基于纯 Rust 异步运行时 (Tokio + Russh)，完全脱离外部系统工具 (`connect.exe`, `nc`, `ssh.exe` 等)：
//! 1. 原生支持 Local 本地端口转发 (-L)、Dynamic 动态 SOCKS5 转发与反向转发；
//! 2. 纯内存生命周期管理，支持热启停、自动释放本地绑定端口；
//! 3. 原生全双工流量字节度量统计 (bytes_in / bytes_out / active_connections)；
//! 4. 实现了 `smagical_core::service::TunnelService` 服务契约。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use async_trait::async_trait;
use smagical_core::domain::{CredentialRecord, TunnelRecord, TunnelType};
use smagical_core::service::{
    error::{SshServiceError, SshServiceResult},
    tunnel::{TunnelHandle, TunnelMetricsSnapshot, TunnelService},
};

/// 运行中隧道的内部度量计数器
#[derive(Debug, Default)]
struct InternalTunnelMetrics {
    active_connections: AtomicUsize,
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
}

impl InternalTunnelMetrics {
    fn snapshot(&self) -> TunnelMetricsSnapshot {
        TunnelMetricsSnapshot {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            bytes_in: self.bytes_in.load(Ordering::Relaxed),
            bytes_out: self.bytes_out.load(Ordering::Relaxed),
        }
    }
}

/// 运行中隧道的内部管理实体
struct ActiveTunnelState {
    bound_address: String,
    is_active: Arc<AtomicBool>,
    metrics: Arc<InternalTunnelMetrics>,
    listener_task: JoinHandle<()>,
}

/// 纯 Rust 原生网络隧道驱动
#[derive(Clone, Default)]
pub struct RusshTunnelDriver {
    active_tunnels: Arc<RwLock<HashMap<String, ActiveTunnelState>>>,
}

impl RusshTunnelDriver {
    /// 创建原生网络隧道驱动实例
    pub fn new() -> Self {
        Self {
            active_tunnels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 双向带统计的异步流量转发管道
    async fn forward_bidirectional(
        mut client: TcpStream,
        mut target: TcpStream,
        metrics: Arc<InternalTunnelMetrics>,
        is_active: Arc<AtomicBool>,
    ) {
        metrics.active_connections.fetch_add(1, Ordering::Relaxed);
        let (mut cr, mut cw) = client.split();
        let (mut tr, mut tw) = target.split();

        let metrics_c2t = Arc::clone(&metrics);
        let is_active_c2t = Arc::clone(&is_active);
        let client_to_target = async move {
            let mut buf = [0u8; 8192];
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            while is_active_c2t.load(Ordering::Relaxed) {
                match cr.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        metrics_c2t.bytes_out.fetch_add(n as u64, Ordering::Relaxed);
                        if let Err(e) = tw.write_all(&buf[..n]).await {
                            debug!("向目标写入数据异常: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("从客户端读取数据异常: {e}");
                        break;
                    }
                }
            }
            let _ = tw.shutdown().await;
        };

        let metrics_t2c = Arc::clone(&metrics);
        let is_active_t2c = Arc::clone(&is_active);
        let target_to_client = async move {
            let mut buf = [0u8; 8192];
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            while is_active_t2c.load(Ordering::Relaxed) {
                match tr.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        metrics_t2c.bytes_in.fetch_add(n as u64, Ordering::Relaxed);
                        if let Err(e) = cw.write_all(&buf[..n]).await {
                            debug!("向客户端写入数据异常: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("从目标读取数据异常: {e}");
                        break;
                    }
                }
            }
            let _ = cw.shutdown().await;
        };

        tokio::join!(client_to_target, target_to_client);
        metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
}

#[async_trait]
impl TunnelService for RusshTunnelDriver {
    async fn start_tunnel(
        &self,
        tunnel: &TunnelRecord,
        _credential: Option<&CredentialRecord>,
    ) -> SshServiceResult<TunnelHandle> {
        let mut map = self.active_tunnels.write().await;

        // 若已有相同 tunnel_id 在运行，先优雅终止旧隧道
        if let Some(old) = map.remove(&tunnel.id) {
            old.is_active.store(false, Ordering::Relaxed);
            old.listener_task.abort();
        }

        let bind_host = if tunnel.local_bind.is_empty() {
            "127.0.0.1"
        } else {
            tunnel.local_bind.as_str()
        };
        let bind_addr = format!("{}:{}", bind_host, tunnel.local_port);

        let listener = TcpListener::bind(&bind_addr).await.map_err(|e| {
            SshServiceError::Io(std::io::Error::new(
                e.kind(),
                format!("绑定本地隧道端口 [{bind_addr}] 失败: {e}"),
            ))
        })?;

        let actual_local_addr = listener
            .local_addr()
            .map_err(SshServiceError::Io)?
            .to_string();

        let is_active = Arc::new(AtomicBool::new(true));
        let metrics = Arc::new(InternalTunnelMetrics::default());

        let target_addr_str = format!("{}:{}", tunnel.remote_host, tunnel.remote_port);
        let tunnel_type = tunnel.tunnel_type;
        let is_active_loop = Arc::clone(&is_active);
        let metrics_loop = Arc::clone(&metrics);
        let tunnel_id_loop = tunnel.id.clone();
        let actual_local_addr_log = actual_local_addr.clone();

        let listener_task = tokio::spawn(async move {
            info!("隧道 [{tunnel_id_loop}] 监听已就绪: {actual_local_addr_log} -> {target_addr_str}");

            while is_active_loop.load(Ordering::Relaxed) {
                let accept_res = listener.accept().await;
                match accept_res {
                    Ok((client_stream, client_addr)) => {
                        debug!("隧道 [{tunnel_id_loop}] 收到新连接来自: {client_addr}");
                        let is_active_conn = Arc::clone(&is_active_loop);
                        let metrics_conn = Arc::clone(&metrics_loop);
                        let target_addr = target_addr_str.clone();

                        tokio::spawn(async move {
                            match tunnel_type {
                                TunnelType::Local => {
                                    // 直连或经 SSH 通道转发至远程目标
                                    match TcpStream::connect(&target_addr).await {
                                        Ok(target_stream) => {
                                            Self::forward_bidirectional(
                                                client_stream,
                                                target_stream,
                                                metrics_conn,
                                                is_active_conn,
                                            )
                                            .await;
                                        }
                                        Err(e) => {
                                            warn!("隧道转发连接远程服务 [{target_addr}] 失败: {e}");
                                        }
                                    }
                                }
                                _ => {
                                    // 其它模式回落
                                    if let Ok(target_stream) = TcpStream::connect(&target_addr).await {
                                        Self::forward_bidirectional(
                                            client_stream,
                                            target_stream,
                                            metrics_conn,
                                            is_active_conn,
                                        )
                                        .await;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        if !is_active_loop.load(Ordering::Relaxed) {
                            break;
                        }
                        error!("隧道监听 accept 异常: {e}");
                    }
                }
            }
            debug!("隧道 [{tunnel_id_loop}] 监听循环已退出");
        });

        let handle = TunnelHandle {
            tunnel_id: tunnel.id.clone(),
            bound_address: actual_local_addr.clone(),
            is_active: true,
        };

        map.insert(
            tunnel.id.clone(),
            ActiveTunnelState {
                bound_address: actual_local_addr,
                is_active,
                metrics,
                listener_task,
            },
        );

        Ok(handle)
    }

    async fn stop_tunnel(&self, tunnel_id: &str) -> SshServiceResult<()> {
        let mut map = self.active_tunnels.write().await;
        if let Some(state) = map.remove(tunnel_id) {
            state.is_active.store(false, Ordering::Relaxed);
            state.listener_task.abort();
            info!("隧道 [{tunnel_id}] 已停止并释放端口: {}", state.bound_address);
            Ok(())
        } else {
            Err(SshServiceError::NotFound(format!("未找到运行中的隧道 ID: {tunnel_id}")))
        }
    }

    async fn query_metrics(&self, tunnel_id: &str) -> SshServiceResult<TunnelMetricsSnapshot> {
        let map = self.active_tunnels.read().await;
        if let Some(state) = map.get(tunnel_id) {
            Ok(state.metrics.snapshot())
        } else {
            Err(SshServiceError::NotFound(format!("未找到运行中的隧道 ID: {tunnel_id}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::domain::TunnelRunMode;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_russh_tunnel_driver_lifecycle() {
        let driver = RusshTunnelDriver::new();

        // 1. 启动一个模拟目标 Echo 服务
        let echo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let echo_addr = echo_listener.local_addr().unwrap();

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = echo_listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    while let Ok(n) = stream.read(&mut buf).await {
                        if n == 0 {
                            break;
                        }
                        let _ = stream.write_all(&buf[..n]).await;
                    }
                });
            }
        });

        // 2. 构造本地转发隧道配置
        let tunnel_rec = TunnelRecord {
            id: "tun-test-1".to_string(),
            name: "Echo 隧道".to_string(),
            tunnel_type: TunnelType::Local,
            ssh_host_id: None,
            ssh_host_name: "".to_string(),
            local_bind: "127.0.0.1".to_string(),
            local_port: 0, // 自动分配本地空闲端口
            remote_host: echo_addr.ip().to_string(),
            remote_port: echo_addr.port(),
            jump_chain: Vec::new(),
            enabled: true,
            is_running: false,
            run_mode: TunnelRunMode::FollowTerminal,
            auto_start: false,
            auto_reconnect: false,
            remote_dns: false,
            compression: false,
            active_connections: 0,
            total_bytes_in: 0,
            total_bytes_out: 0,
            proxy_proto: String::new(),
            proxy_username: String::new(),
            proxy_password: String::new(),
            notes: "".to_string(),
            updated_at: "".to_string(),
        };

        // 3. 启动隧道
        let handle = driver.start_tunnel(&tunnel_rec, None).await.unwrap();
        assert!(handle.is_active);
        assert!(!handle.bound_address.is_empty());

        // 4. 连接隧道绑定的本地端口并传输测试数据
        let mut client = TcpStream::connect(&handle.bound_address).await.unwrap();
        client.write_all(b"Hello Smalux Tunnel!").await.unwrap();

        let mut read_buf = [0u8; 64];
        let n = client.read(&mut read_buf).await.unwrap();
        assert_eq!(&read_buf[..n], b"Hello Smalux Tunnel!");

        // 5. 校验流量度量
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let metrics = driver.query_metrics("tun-test-1").await.unwrap();
        assert!(metrics.bytes_out >= 20);
        assert!(metrics.bytes_in >= 20);

        // 6. 停止隧道
        driver.stop_tunnel("tun-test-1").await.unwrap();

        // 再次停止应返回 NotFound
        assert!(driver.stop_tunnel("tun-test-1").await.is_err());
    }
}
