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

    /// 预检本地端口可用性并提供占用冲突诊断建议
    pub fn check_local_port_availability(bind_host: &str, port: u16) -> SshServiceResult<()> {
        if port == 0 {
            return Ok(()); // 0 表示由系统自动分配空闲端口
        }
        let host = if bind_host.is_empty() { "127.0.0.1" } else { bind_host };
        let bind_addr = format!("{host}:{port}");

        match std::net::TcpListener::bind(&bind_addr) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                #[cfg(windows)]
                let tip = format!(
                    "本地端口 [{bind_addr}] 已被系统其他进程占用！\n排查建议：请在终端执行 'netstat -ano | findstr :{port}' 查询占用该端口的进程 PID，并在任务管理器或命令行中释放该端口，或修改规则使用其他空闲端口。"
                );
                #[cfg(not(windows))]
                let tip = format!(
                    "本地端口 [{bind_addr}] 已被系统其他进程占用！\n排查建议：请在终端执行 'lsof -i :{port}' 或 'ss -tulpn | grep :{port}' 查询占用进程并释放端口，或修改规则使用其他空闲端口。"
                );
                Err(SshServiceError::Io(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    tip,
                )))
            }
            Err(e) => Err(SshServiceError::Io(std::io::Error::new(
                e.kind(),
                format!("预检本地端口 [{bind_addr}] 绑定失败: {e}"),
            ))),
        }
    }

    /// 处理标准 SOCKS5 代理连接协商与出网转发 (RFC 1928 / RFC 1929)
    async fn handle_socks5_connection(
        mut client: TcpStream,
        auth_opt: Option<(String, String)>,
        metrics: Arc<InternalTunnelMetrics>,
        is_active: Arc<AtomicBool>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // 1. 协商认证方法 (RFC 1928 Section 3)
        let mut ver_methods = [0u8; 2];
        if client.read_exact(&mut ver_methods).await.is_err() {
            return;
        }
        if ver_methods[0] != 0x05 {
            return;
        }
        let nmethods = ver_methods[1] as usize;
        let mut methods = vec![0u8; nmethods];
        if client.read_exact(&mut methods).await.is_err() {
            return;
        }

        // 2. 身份认证判定
        if let Some((ref req_user, ref req_pass)) = auth_opt {
            if !req_user.is_empty() {
                // 要求账号密码认证 (0x02)
                if !methods.contains(&0x02) {
                    let _ = client.write_all(&[0x05, 0xFF]).await;
                    return;
                }
                if client.write_all(&[0x05, 0x02]).await.is_err() {
                    return;
                }
                // RFC 1929 用户名/密码子协商
                let mut sub_ver = [0u8; 2];
                if client.read_exact(&mut sub_ver).await.is_err() || sub_ver[0] != 0x01 {
                    let _ = client.write_all(&[0x01, 0x01]).await;
                    return;
                }
                let ulen = sub_ver[1] as usize;
                let mut ubuf = vec![0u8; ulen];
                if client.read_exact(&mut ubuf).await.is_err() {
                    return;
                }
                let mut plen_buf = [0u8; 1];
                if client.read_exact(&mut plen_buf).await.is_err() {
                    return;
                }
                let plen = plen_buf[0] as usize;
                let mut pbuf = vec![0u8; plen];
                if client.read_exact(&mut pbuf).await.is_err() {
                    return;
                }
                let uname = String::from_utf8_lossy(&ubuf);
                let passwd = String::from_utf8_lossy(&pbuf);
                if uname != *req_user || passwd != *req_pass {
                    let _ = client.write_all(&[0x01, 0x01]).await;
                    return;
                }
                // 认证成功响应
                if client.write_all(&[0x01, 0x00]).await.is_err() {
                    return;
                }
            } else {
                // 无需认证 (0x00)
                if !methods.contains(&0x00) {
                    let _ = client.write_all(&[0x05, 0xFF]).await;
                    return;
                }
                if client.write_all(&[0x05, 0x00]).await.is_err() {
                    return;
                }
            }
        } else {
            // 无需认证 (0x00)
            if !methods.contains(&0x00) {
                let _ = client.write_all(&[0x05, 0xFF]).await;
                return;
            }
            if client.write_all(&[0x05, 0x00]).await.is_err() {
                return;
            }
        }

        // 3. 读取请求命令 (RFC 1928 Section 4)
        let mut req_header = [0u8; 4];
        if client.read_exact(&mut req_header).await.is_err() {
            return;
        }
        if req_header[0] != 0x05 {
            return;
        }
        if req_header[1] != 0x01 {
            // 仅支持 0x01 (CONNECT)
            let _ = client.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await;
            return;
        }

        let target_host = match req_header[3] {
            0x01 => {
                // IPv4: 4 字节
                let mut ip_buf = [0u8; 4];
                if client.read_exact(&mut ip_buf).await.is_err() {
                    return;
                }
                std::net::Ipv4Addr::from(ip_buf).to_string()
            }
            0x03 => {
                // 域名: 1 字节长度 + 域名字符串
                let mut dlen_buf = [0u8; 1];
                if client.read_exact(&mut dlen_buf).await.is_err() {
                    return;
                }
                let mut dbuf = vec![0u8; dlen_buf[0] as usize];
                if client.read_exact(&mut dbuf).await.is_err() {
                    return;
                }
                String::from_utf8_lossy(&dbuf).to_string()
            }
            0x04 => {
                // IPv6: 16 字节
                let mut ip_buf = [0u8; 16];
                if client.read_exact(&mut ip_buf).await.is_err() {
                    return;
                }
                std::net::Ipv6Addr::from(ip_buf).to_string()
            }
            _ => {
                // 不支持的地址类型 (0x08)
                let _ = client.write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await;
                return;
            }
        };

        let mut port_buf = [0u8; 2];
        if client.read_exact(&mut port_buf).await.is_err() {
            return;
        }
        let target_port = u16::from_be_bytes(port_buf);
        let target_addr = format!("{target_host}:{target_port}");

        // 4. 连接远程目标 (带 10 秒超时)
        match tokio::time::timeout(std::time::Duration::from_secs(10), TcpStream::connect(&target_addr)).await {
            Ok(Ok(target_stream)) => {
                // 连接成功，响应 0x00 (SUCCESS)
                if client.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await.is_err() {
                    return;
                }
                // 启动全双工管道
                Self::forward_bidirectional(client, target_stream, metrics, is_active).await;
            }
            Ok(Err(e)) => {
                debug!("SOCKS5 代理连接远程目标 [{target_addr}] 失败: {e}");
                // 0x05: Connection Refused
                let _ = client.write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await;
            }
            Err(_) => {
                debug!("SOCKS5 代理连接远程目标 [{target_addr}] 超时");
                // 0x04: Host Unreachable
                let _ = client.write_all(&[0x05, 0x04, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await;
            }
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
            // 短暂让出执行权确保旧套接字已完成物理释放
            tokio::task::yield_now().await;
        }

        let bind_host = if tunnel.local_bind.is_empty() {
            "127.0.0.1"
        } else {
            tunnel.local_bind.as_str()
        };
        let bind_addr = format!("{}:{}", bind_host, tunnel.local_port);

        // 端口占用预检与冲突防护
        Self::check_local_port_availability(bind_host, tunnel.local_port)?;

        let listener = TcpListener::bind(&bind_addr).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                #[cfg(windows)]
                let tip = format!(
                    "本地端口 [{bind_addr}] 绑定冲突已被占用！\n排查建议：在终端执行 'netstat -ano | findstr :{}' 查看占用进程 PID 并释放端口。",
                    tunnel.local_port
                );
                #[cfg(not(windows))]
                let tip = format!(
                    "本地端口 [{bind_addr}] 绑定冲突已被占用！\n排查建议：在终端执行 'lsof -i :{}' 查看占用进程并释放端口。",
                    tunnel.local_port
                );
                SshServiceError::Io(std::io::Error::new(std::io::ErrorKind::AddrInUse, tip))
            } else {
                SshServiceError::Io(std::io::Error::new(
                    e.kind(),
                    format!("绑定本地隧道端口 [{bind_addr}] 失败: {e}"),
                ))
            }
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
        let auth_opt = if !tunnel.proxy_username.is_empty() {
            Some((tunnel.proxy_username.clone(), tunnel.proxy_password.clone()))
        } else {
            None
        };

        let listener_task = tokio::spawn(async move {
            info!("隧道 [{tunnel_id_loop}] 监听已就绪: {actual_local_addr_log} -> {target_addr_str} (类型: {:?})", tunnel_type);

            while is_active_loop.load(Ordering::Relaxed) {
                let accept_res = listener.accept().await;
                match accept_res {
                    Ok((client_stream, client_addr)) => {
                        debug!("隧道 [{tunnel_id_loop}] 收到新连接来自: {client_addr}");
                        let is_active_conn = Arc::clone(&is_active_loop);
                        let metrics_conn = Arc::clone(&metrics_loop);
                        let target_addr = target_addr_str.clone();
                        let auth_opt_conn = auth_opt.clone();

                        tokio::spawn(async move {
                            match tunnel_type {
                                TunnelType::Dynamic => {
                                    // 动态 SOCKS5 代理网关协议处理
                                    Self::handle_socks5_connection(
                                        client_stream,
                                        auth_opt_conn,
                                        metrics_conn,
                                        is_active_conn,
                                    )
                                    .await;
                                }
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

    async fn is_tunnel_alive(&self, tunnel_id: &str) -> bool {
        let map = self.active_tunnels.read().await;
        if let Some(state) = map.get(tunnel_id) {
            state.is_active.load(Ordering::Relaxed) && !state.listener_task.is_finished()
        } else {
            false
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

    #[tokio::test]
    async fn test_check_local_port_availability() {
        // 1. 端口 0 永远可用
        assert!(RusshTunnelDriver::check_local_port_availability("127.0.0.1", 0).is_ok());

        // 2. 先主动占用一个本地随机端口
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = occupied.local_addr().unwrap().port();

        // 3. 预检该端口应返回冲突错误，且错误提示包含诊断排查建议
        let res = RusshTunnelDriver::check_local_port_availability("127.0.0.1", port);
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("已经被系统其他进程占用") || err_msg.contains("排查建议"));

        // 4. 释放套接字后重测
        drop(occupied);
        let res2 = RusshTunnelDriver::check_local_port_availability("127.0.0.1", port);
        assert!(res2.is_ok());
    }

    #[tokio::test]
    async fn test_socks5_dynamic_tunnel() {
        let driver = RusshTunnelDriver::new();

        // 1. 模拟后端目标 Echo 服务
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

        // 2. 启动动态 SOCKS5 代理网关 (-D)
        let socks_rec = TunnelRecord {
            id: "tun-socks5-1".to_string(),
            name: "SOCKS5 代理网关".to_string(),
            tunnel_type: TunnelType::Dynamic,
            ssh_host_id: None,
            ssh_host_name: "".to_string(),
            local_bind: "127.0.0.1".to_string(),
            local_port: 0,
            remote_host: "".to_string(),
            remote_port: 0,
            jump_chain: Vec::new(),
            enabled: true,
            is_running: false,
            run_mode: TunnelRunMode::FollowApp,
            auto_start: false,
            auto_reconnect: true,
            remote_dns: false,
            compression: false,
            active_connections: 0,
            total_bytes_in: 0,
            total_bytes_out: 0,
            proxy_proto: "SOCKS5".to_string(),
            proxy_username: "".to_string(),
            proxy_password: "".to_string(),
            notes: "".to_string(),
            updated_at: "".to_string(),
        };

        let handle = driver.start_tunnel(&socks_rec, None).await.unwrap();
        assert!(driver.is_tunnel_alive("tun-socks5-1").await);

        // 3. 客户端连接 SOCKS5 代理并进行 RFC 1928 握手
        let mut client = TcpStream::connect(&handle.bound_address).await.unwrap();

        // 握手问候: [VER 5, NMETHODS 1, METHOD 0]
        client.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
        let mut auth_resp = [0u8; 2];
        client.read_exact(&mut auth_resp).await.unwrap();
        assert_eq!(auth_resp, [0x05, 0x00]); // 成功协商为无认证

        // 发送 CONNECT 请求至 Echo 服务
        let mut req = vec![0x05, 0x01, 0x00, 0x01]; // VER 5, CMD 1, RSV 0, ATYP 1 (IPv4)
        req.extend_from_slice(&[127, 0, 0, 1]);
        req.extend_from_slice(&echo_addr.port().to_be_bytes());
        client.write_all(&req).await.unwrap();

        let mut req_resp = [0u8; 10];
        client.read_exact(&mut req_resp).await.unwrap();
        assert_eq!(req_resp[0], 0x05);
        assert_eq!(req_resp[1], 0x00); // 0x00 = SUCCESS

        // 发送真实数据通过 SOCKS5 转发到 Echo 服务
        client.write_all(b"Hello SOCKS5 Dynamic Tunnel!").await.unwrap();
        let mut echo_buf = [0u8; 64];
        let n = client.read(&mut echo_buf).await.unwrap();
        assert_eq!(&echo_buf[..n], b"Hello SOCKS5 Dynamic Tunnel!");

        // 4. 停止隧道并校验 alive 状态
        driver.stop_tunnel("tun-socks5-1").await.unwrap();
        assert!(!driver.is_tunnel_alive("tun-socks5-1").await);
    }
}
