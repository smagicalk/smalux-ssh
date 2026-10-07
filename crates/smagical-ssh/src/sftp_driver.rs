//! 基于 russh-sftp 的纯 Rust 原生 SFTP 客户端驱动实现。
//!
//! 实现 `smagical_core::service::SftpService` 契约，彻底脱离系统平台外部命令与进程调用。

use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use tracing::{debug, info};

use smagical_core::domain::file_item::format_file_size;
use smagical_core::domain::{FileItemData, HostRecord};
use smagical_core::service::{SftpService, SshServiceError, SshServiceResult, TransferProgress};

/// 原生 SFTP 客户端事件处理器 (支持 TOFU known_hosts 校验与安全防劫持)
#[derive(Clone)]
struct SftpClientHandler {
    host: String,
    port: u16,
    policy: String,
    known_hosts_path: Option<std::path::PathBuf>,
}

impl SftpClientHandler {
    fn new(host: &str, port: u16, policy: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            policy: policy.to_string(),
            known_hosts_path: crate::known_hosts::get_default_known_hosts_path(),
        }
    }
}

impl russh::client::Handler for SftpClientHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let host = self.host.clone();
        let port = self.port;
        let policy = self.policy.clone();
        let path_opt = self.known_hosts_path.clone();

        async move {
            if policy == "insecure-accept-all" {
                return Ok(true);
            }

            let Some(known_hosts_path) = path_opt else {
                return Ok(true);
            };

            match crate::known_hosts::verify_server_key_in_file(&known_hosts_path, &host, port, server_public_key) {
                Ok(smagical_core::service::HostKeyVerificationResult::Trusted) => {
                    info!(target: "smagical_ssh::sftp", "SFTP 主机公钥验真通过 (已在 known_hosts 信任白名单): {}:{}", host, port);
                    Ok(true)
                }
                Ok(smagical_core::service::HostKeyVerificationResult::FirstTimeHost { fingerprint, public_key_text: _ }) => {
                    if policy == "strict" {
                        tracing::warn!(target: "smagical_ssh::sftp", "严格模式拒绝首次连接的未登记 SFTP 主机: {}:{} (指纹: {})", host, port, fingerprint);
                        Ok(false)
                    } else {
                        info!(target: "smagical_ssh::sftp", "首次连接 SFTP 主机 {}:{}，指纹 [{}]，正在安全登记至 known_hosts...", host, port, fingerprint);
                        let _ = crate::known_hosts::append_known_host_to_file(&known_hosts_path, &host, port, server_public_key);
                        Ok(true)
                    }
                }
                Ok(smagical_core::service::HostKeyVerificationResult::Mismatch { expected_fingerprint, actual_fingerprint }) => {
                    tracing::error!(
                        target: "smagical_ssh::security",
                        "🚨【严重安全警报】SFTP 目标主机公钥与已知历史记录不匹配！疑似遭遇中间人劫持攻击！主机: {}:{}, 历史记录指纹: {}, 本次接收指纹: {}",
                        host, port, expected_fingerprint, actual_fingerprint
                    );
                    Ok(false)
                }
                Err(e) => {
                    tracing::warn!(target: "smagical_ssh::sftp", "读取 known_hosts 异常: {:?}，回退放行", e);
                    Ok(true)
                }
            }
        }
    }
}

/// 纯 Rust 原生 SFTP 驱动引擎 (RusshSftpDriver)
#[derive(Clone)]
pub struct RusshSftpDriver {
    /// 活跃的 SFTP 会话池映射 (session_id -> Arc<SftpSession>)
    sessions: Arc<RwLock<HashMap<String, Arc<russh_sftp::client::SftpSession>>>>,
    known_hosts_path: Option<std::path::PathBuf>,
}

impl Default for RusshSftpDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl RusshSftpDriver {
    /// 创建全新的纯 Rust SFTP 驱动管理器
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            known_hosts_path: crate::known_hosts::get_default_known_hosts_path(),
        }
    }

    /// 使用指定的 known_hosts 物理路径创建 SFTP 驱动管理器
    pub fn with_known_hosts_path(path: Option<std::path::PathBuf>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            known_hosts_path: path,
        }
    }

    /// 使用真实主机网络参数与凭据握手建立纯 Rust SFTP 会话并注册到会话池
    pub async fn connect_and_register(
        &self,
        session_id: &str,
        host: &HostRecord,
        username: &str,
        password: Option<&str>,
        private_key_pem: Option<&str>,
    ) -> SshServiceResult<()> {
        let proxy_info = host.proxy_type.as_deref().filter(|&p| p != "direct" && !p.is_empty())
            .zip(host.proxy_host.as_deref().filter(|&h| !h.is_empty()))
            .map(|(t, h)| (t, h, host.proxy_port.unwrap_or(0)));

        self.connect_raw(
            session_id,
            &host.address,
            host.port,
            username,
            password,
            private_key_pem,
            host.jump_chain_summary.as_deref(),
            proxy_info,
        ).await
    }

    /// 使用 SshLaunchConfig 快速建立纯 Rust 原生 SFTP 会话并注入会话池 (用于连接复用)
    pub async fn connect_and_register_with_config(
        &self,
        session_id: &str,
        config: &crate::ssh_config::SshLaunchConfig,
    ) -> SshServiceResult<()> {
        let username = config.username.as_deref().unwrap_or("root");
        let proxy_info = config.proxy_type.as_deref().filter(|&p| p != "direct" && !p.is_empty())
            .zip(config.proxy_host.as_deref().filter(|&h| !h.is_empty()))
            .map(|(t, h)| (t, h, config.proxy_port.unwrap_or(0)));

        self.connect_raw(
            session_id,
            &config.host,
            config.port,
            username,
            config.password.as_deref(),
            config.private_key_pem.as_deref(),
            config.jump_host.as_deref(),
            proxy_info,
        ).await
    }

    /// 内部认证 Handle 辅助方法
    async fn authenticate_raw_handle(
        handle: &mut russh::client::Handle<SftpClientHandler>,
        username: &str,
        password: Option<&str>,
        private_key_pem: Option<&str>,
    ) -> SshServiceResult<()> {
        let mut authenticated = false;

        if let Some(key_pem) = private_key_pem {
            if !key_pem.trim().is_empty() {
                debug!(target: "smagical_ssh::sftp", "正在尝试私钥证书认证...");
                if let Ok(key) = russh::keys::decode_secret_key(key_pem, None) {
                    let key_with_alg = russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None);
                    if let Ok(auth_res) = handle.authenticate_publickey(username, key_with_alg).await {
                        if auth_res.success() {
                            authenticated = true;
                            info!(target: "smagical_ssh::sftp", "私钥公钥对认证成功");
                        }
                    }
                }
            }
        }

        if !authenticated {
            if let Some(pwd) = password {
                debug!(target: "smagical_ssh::sftp", "正在尝试密码认证...");
                let auth_res = handle
                    .authenticate_password(username, pwd)
                    .await
                    .map_err(|e| SshServiceError::AuthFailed(format!("密码认证通信异常: {}", e)))?;
                if auth_res.success() {
                    authenticated = true;
                    info!(target: "smagical_ssh::sftp", "远程密码认证成功");
                }
            }
        }

        if !authenticated {
            return Err(SshServiceError::AuthFailed(format!("用户 [{}] 凭据认证失败", username)));
        }
        Ok(())
    }

    /// 统一底层 SFTP 连接与鉴权实现，支持直连、SOCKS5/HTTP 代理及跳板机多跳链路
    #[allow(clippy::too_many_arguments)]
    async fn connect_raw(
        &self,
        session_id: &str,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        private_key_pem: Option<&str>,
        jump_chain: Option<&str>,
        proxy: Option<(&str, &str, u16)>,
    ) -> SshServiceResult<()> {
        info!(
            target: "smagical_ssh::sftp",
            "正在发起原生纯 Rust SFTP 连接握手: {}@{}:{}",
            username, host, port
        );

        let mut client_config = russh::client::Config::default();
        client_config.keepalive_interval = Some(std::time::Duration::from_secs(15));
        client_config.keepalive_max = 3;
        client_config.inactivity_timeout = None;
        let config = Arc::new(client_config);
        let addr = format!("{}:{}", host, port);
        let timeout_dur = std::time::Duration::from_secs(15);
        let jump_hops = jump_chain.map(crate::session_driver::parse_jump_chain).unwrap_or_default();

        let handle = if !jump_hops.is_empty() {
            let total_hops = jump_hops.len();
            info!(target: "smagical_ssh::sftp", "SFTP 检测到跳板机链路 (共 {} 跳): {:?}", total_hops, jump_hops);
            let first_hop = &jump_hops[0];
            let first_addr = format!("{}:{}", first_hop.host, first_hop.port);
            let first_user = first_hop.username.as_deref().unwrap_or(username);
            let mut h_first = SftpClientHandler::new(&first_hop.host, first_hop.port, "accept_new");
            h_first.known_hosts_path = self.known_hosts_path.clone();

            let mut current_handle = tokio::time::timeout(
                timeout_dur,
                russh::client::connect(config.clone(), first_addr.as_str(), h_first)
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("SFTP 连接第 1/{} 跳跳板机超时 [{}]", total_hops, first_addr)))?
            .map_err(|e| SshServiceError::HostUnreachable(format!("SFTP 连接第 1/{} 跳跳板机失败 [{}]: {}", total_hops, first_addr, e)))?;

            Self::authenticate_raw_handle(&mut current_handle, first_user, password, private_key_pem).await?;

            for (idx, hop) in jump_hops[1..].iter().enumerate() {
                let hop_num = idx + 2;
                let hop_addr = format!("{}:{}", hop.host, hop.port);
                let hop_user = hop.username.as_deref().unwrap_or(username);
                let channel = tokio::time::timeout(
                    timeout_dur,
                    current_handle.channel_open_direct_tcpip(&hop.host, hop.port as u32, "127.0.0.1", 0)
                )
                .await
                .map_err(|_| SshServiceError::Timeout(format!("SFTP 请求建立 Direct-TCPIP 隧道至第 {}/{} 跳 [{}] 超时", hop_num, total_hops, hop_addr)))?
                .map_err(|e| SshServiceError::ProtocolError(format!("SFTP 跳板机转发至第 {}/{} 跳 [{}] 失败: {}", hop_num, total_hops, hop_addr, e)))?;

                let mut h_hop = SftpClientHandler::new(&hop.host, hop.port, "accept_new");
                h_hop.known_hosts_path = self.known_hosts_path.clone();
                let mut next_handle = tokio::time::timeout(
                    timeout_dur,
                    russh::client::connect_stream(config.clone(), channel.into_stream(), h_hop)
                )
                .await
                .map_err(|_| SshServiceError::Timeout(format!("SFTP 通过隧道握手中间跳板机 [{}] 超时", hop_addr)))?
                .map_err(|e| SshServiceError::HostUnreachable(format!("SFTP 通过隧道连接中间跳板机 [{}] 失败: {}", hop_addr, e)))?;

                Self::authenticate_raw_handle(&mut next_handle, hop_user, password, private_key_pem).await?;
                current_handle = next_handle;
            }

            let target_channel = tokio::time::timeout(
                timeout_dur,
                current_handle.channel_open_direct_tcpip(host, port as u32, "127.0.0.1", 0)
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("SFTP 建立通往目标主机 [{}] 的 Direct-TCPIP 隧道超时", addr)))?
            .map_err(|e| SshServiceError::ProtocolError(format!("SFTP 跳板机建立通往目标主机 [{}] 的 Direct-TCPIP 隧道失败: {}", addr, e)))?;

            let mut h_target = SftpClientHandler::new(host, port, "accept_new");
            h_target.known_hosts_path = self.known_hosts_path.clone();
            let mut target_handle = tokio::time::timeout(
                timeout_dur,
                russh::client::connect_stream(config.clone(), target_channel.into_stream(), h_target)
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("SFTP 通过跳板机直连目标主机 [{}] 握手超时", addr)))?
            .map_err(|e| SshServiceError::HostUnreachable(format!("SFTP 通过跳板机直连目标主机 [{}] 失败: {}", addr, e)))?;

            Self::authenticate_raw_handle(&mut target_handle, username, password, private_key_pem).await?;
            target_handle
        } else if let Some((ptype, phost, pport)) = proxy {
            let proxy_addr = format!("{}:{}", phost, pport);
            let stream = if ptype == "socks5" || ptype == "socks" {
                tokio::time::timeout(timeout_dur, crate::session_driver::connect_via_socks5(&proxy_addr, host, port))
                    .await
                    .map_err(|_| SshServiceError::Timeout(format!("SFTP 连接 SOCKS5 代理超时 [{}]", proxy_addr)))?
                    .map_err(|e| SshServiceError::HostUnreachable(format!("SFTP SOCKS5 代理穿透失败 [{}]: {}", proxy_addr, e)))?
            } else {
                tokio::time::timeout(timeout_dur, crate::session_driver::connect_via_http_connect(&proxy_addr, host, port))
                    .await
                    .map_err(|_| SshServiceError::Timeout(format!("SFTP 连接 HTTP 代理超时 [{}]", proxy_addr)))?
                    .map_err(|e| SshServiceError::HostUnreachable(format!("SFTP HTTP 代理穿透失败 [{}]: {}", proxy_addr, e)))?
            };
            let mut handler = SftpClientHandler::new(host, port, "accept_new");
            handler.known_hosts_path = self.known_hosts_path.clone();
            let mut h = tokio::time::timeout(
                timeout_dur,
                russh::client::connect_stream(config.clone(), stream, handler)
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("SFTP 通过代理连接目标主机超时: [{}]", addr)))?
            .map_err(|e| SshServiceError::HostUnreachable(format!("SFTP 通过代理连接目标 [{}] 失败: {}", addr, e)))?;

            Self::authenticate_raw_handle(&mut h, username, password, private_key_pem).await?;
            h
        } else {
            let mut handler = SftpClientHandler::new(host, port, "accept_new");
            handler.known_hosts_path = self.known_hosts_path.clone();
            let mut h = tokio::time::timeout(
                timeout_dur,
                russh::client::connect(config, addr.as_str(), handler)
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("连接远程 SFTP 主机超时 (15s): {}", addr)))?
            .map_err(|e| SshServiceError::HostUnreachable(format!("连接远程主机失败: {}", e)))?;

            Self::authenticate_raw_handle(&mut h, username, password, private_key_pem).await?;
            h
        };

        // 打开 SFTP 子系统通道
        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("打开 Session Channel 失败: {}", e)))?;

        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("请求 sftp 子系统失败: {}", e)))?;

        let sftp_session = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("初始化 SFTP 协议流失败: {}", e)))?;

        {
            let mut guard = self.sessions.write().await;
            guard.insert(session_id.to_string(), Arc::new(sftp_session));
        }

        info!(target: "smagical_ssh::sftp", "纯 Rust SFTP 会话注册成功: [{}]", session_id);
        Ok(())
    }

    /// 检查指定 session_id 是否已有活跃 SFTP 会话
    pub async fn has_session(&self, session_id: &str) -> bool {
        let guard = self.sessions.read().await;
        guard.contains_key(session_id)
    }

    /// 移除已失效的 SFTP 会话
    pub async fn remove_session(&self, session_id: &str) {
        let mut guard = self.sessions.write().await;
        guard.remove(session_id);
    }

    /// 获取指定 session_id 的克隆会话句柄
    async fn get_session(&self, session_id: &str) -> SshServiceResult<Arc<russh_sftp::client::SftpSession>> {
        let guard = self.sessions.read().await;
        guard
            .get(session_id)
            .cloned()
            .ok_or_else(|| SshServiceError::NotFound(format!("未找到处于活跃状态的 SFTP 会话 [{}]", session_id)))
    }
}

#[async_trait]
impl SftpService for RusshSftpDriver {
    async fn list_dir(&self, session_id: &str, remote_path: &str) -> SshServiceResult<Vec<FileItemData>> {
        let sftp = self.get_session(session_id).await?;
        let norm_path = if remote_path.trim().is_empty() || remote_path == "~" {
            "."
        } else {
            let p = remote_path.trim_end_matches('/');
            if p.is_empty() {
                "/"
            } else {
                p
            }
        };

        let mut read_dir = sftp
            .read_dir(norm_path)
            .await
            .map_err(|e| SshServiceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let mut result = Vec::new();

        while let Some(entry) = read_dir.next() {
            let file_name = entry.file_name();
            if file_name == "." || file_name == ".." || file_name.is_empty() {
                continue;
            }

            let meta = entry.metadata();
            let is_dir = meta.is_dir();
            let is_symlink = meta.is_symlink();
            let size = meta.size.unwrap_or(0);
            let mtime = meta.mtime.unwrap_or(0) as u64;

            let full_path = if norm_path == "." {
                file_name.clone()
            } else if norm_path == "/" {
                format!("/{}", file_name)
            } else {
                format!("{}/{}", norm_path, file_name)
            };

            let is_hidden = file_name.starts_with('.');
            let permissions = format_permissions(meta.permissions);

            result.push(FileItemData {
                id: full_path.clone(),
                name: file_name,
                path: full_path,
                is_dir,
                size: if is_dir { 0 } else { size },
                size_formatted: if is_dir { "-".to_string() } else { format_file_size(size) },
                modified_at: mtime,
                modified_formatted: format_epoch_seconds(mtime),
                permissions,
                owner: meta.uid.map(|u| u.to_string()).unwrap_or_else(|| "user".to_string()),
                group: meta.gid.map(|g| g.to_string()).unwrap_or_else(|| "group".to_string()),
                is_symlink,
                is_hidden,
                is_expanded: false,
                level: 0,
                item_count: 0,
            });
        }

        // 排序：目录优先，名称字典序
        result.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(result)
    }

    async fn stat_path(&self, session_id: &str, remote_path: &str) -> SshServiceResult<FileItemData> {
        let sftp = self.get_session(session_id).await?;
        let meta = sftp
            .metadata(remote_path)
            .await
            .map_err(|e| SshServiceError::NotFound(format!("获取文件元数据失败 [{}]: {}", remote_path, e)))?;

        let is_dir = meta.is_dir();
        let is_symlink = meta.is_symlink();
        let size = meta.size.unwrap_or(0);
        let mtime = meta.mtime.unwrap_or(0) as u64;
        let file_name = remote_path.split(['/', '\\']).last().unwrap_or("file").to_string();

        Ok(FileItemData {
            id: remote_path.to_string(),
            name: file_name.clone(),
            path: remote_path.to_string(),
            is_dir,
            size: if is_dir { 0 } else { size },
            size_formatted: if is_dir { "-".to_string() } else { format_file_size(size) },
            modified_at: mtime,
            modified_formatted: format_epoch_seconds(mtime),
            permissions: format_permissions(meta.permissions),
            owner: meta.uid.map(|u| u.to_string()).unwrap_or_else(|| "user".to_string()),
            group: meta.gid.map(|g| g.to_string()).unwrap_or_else(|| "group".to_string()),
            is_symlink,
            is_hidden: file_name.starts_with('.'),
            is_expanded: false,
            level: 0,
            item_count: 0,
        })
    }

    async fn create_dir(&self, session_id: &str, remote_path: &str) -> SshServiceResult<()> {
        let sftp = self.get_session(session_id).await?;
        sftp.create_dir(remote_path)
            .await
            .map_err(|e| SshServiceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    async fn remove_path(&self, session_id: &str, remote_path: &str, recursive: bool) -> SshServiceResult<()> {
        let sftp = self.get_session(session_id).await?;
        let meta = sftp.metadata(remote_path).await.ok();

        if let Some(m) = meta {
            if m.is_dir() {
                if recursive {
                    // 递归删除子项
                    if let Ok(mut dir_entries) = sftp.read_dir(remote_path).await {
                        while let Some(entry) = dir_entries.next() {
                            let name = entry.file_name();
                            if name == "." || name == ".." {
                                continue;
                            }
                            let sub_path = format!("{}/{}", remote_path.trim_end_matches('/'), name);
                            let _ = Box::pin(self.remove_path(session_id, &sub_path, true)).await;
                        }
                    }
                }
                return sftp
                    .remove_dir(remote_path)
                    .await
                    .map_err(|e| SshServiceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())));
            }
        }

        sftp.remove_file(remote_path)
            .await
            .map_err(|e| SshServiceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    async fn rename(&self, session_id: &str, old_path: &str, new_path: &str) -> SshServiceResult<()> {
        let sftp = self.get_session(session_id).await?;
        sftp.rename(old_path, new_path)
            .await
            .map_err(|e| SshServiceError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    async fn download_file(
        &self,
        session_id: &str,
        remote_path: &str,
        local_path: &str,
        progress_tx: Option<UnboundedSender<TransferProgress>>,
    ) -> SshServiceResult<()> {
        let sftp = self.get_session(session_id).await?;
        let mut remote_file = sftp
            .open(remote_path)
            .await
            .map_err(|e| SshServiceError::NotFound(format!("打开远程文件失败: {}", e)))?;

        let total_size = remote_file
            .metadata()
            .await
            .ok()
            .and_then(|m| m.size)
            .unwrap_or(0);

        if let Some(parent) = Path::new(local_path).parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let mut local_file = tokio::fs::File::create(local_path)
            .await
            .map_err(SshServiceError::Io)?;

        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let chunk_size = 128 * 1024; // 128 KB 分块流水线 (双缓冲并发读写)
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8); // 8 个分块容量背压队列

        let start_time = std::time::Instant::now();
        let reader_fut = async {
            let mut read_buf = vec![0u8; chunk_size];
            loop {
                let bytes_read = remote_file.read(&mut read_buf).await?;
                if bytes_read == 0 {
                    break;
                }
                if tx.send(read_buf[..bytes_read].to_vec()).await.is_err() {
                    break;
                }
            }
            drop(tx);
            Ok::<(), std::io::Error>(())
        };

        let writer_fut = async {
            let mut transferred = 0u64;
            let mut last_progress_send = std::time::Instant::now();
            let mut last_sample_time = std::time::Instant::now();
            let mut last_sample_bytes = 0u64;
            let mut current_speed = 0u64;

            while let Some(chunk) = rx.recv().await {
                local_file.write_all(&chunk).await?;
                transferred += chunk.len() as u64;

                if let Some(ref tx_prog) = progress_tx {
                    let now = std::time::Instant::now();
                    let sample_elapsed = now.duration_since(last_sample_time).as_secs_f64();
                    if sample_elapsed >= 0.2 {
                        let bytes_in_sample = transferred.saturating_sub(last_sample_bytes);
                        let instant_speed = (bytes_in_sample as f64 / sample_elapsed) as u64;
                        current_speed = if current_speed == 0 {
                            instant_speed
                        } else {
                            ((instant_speed as f64 * 0.7) + (current_speed as f64 * 0.3)) as u64
                        };
                        last_sample_time = now;
                        last_sample_bytes = transferred;
                    }

                    if now.duration_since(last_progress_send).as_millis() >= 100 {
                        let _ = tx_prog.send(TransferProgress {
                            task_id: format!("dl-{}", remote_path),
                            transferred_bytes: transferred,
                            total_bytes: total_size,
                            speed_bytes_per_sec: current_speed,
                        });
                        last_progress_send = now;
                    }
                }
            }

            local_file.flush().await?;
            Ok::<u64, std::io::Error>(transferred)
        };

        let (_, transferred) = match tokio::try_join!(reader_fut, writer_fut) {
            Ok(res) => res,
            Err(e) => {
                let _ = tokio::fs::remove_file(local_path).await;
                return Err(SshServiceError::Io(e));
            }
        };

        if let Some(ref tx) = progress_tx {
            let elapsed_secs = start_time.elapsed().as_secs_f64().max(0.001);
            let avg_speed = (transferred as f64 / elapsed_secs) as u64;
            let _ = tx.send(TransferProgress {
                task_id: format!("dl-{}", remote_path),
                transferred_bytes: transferred,
                total_bytes: total_size,
                speed_bytes_per_sec: avg_speed,
            });
        }

        info!(
            target: "smagical_ssh::sftp",
            "纯 Rust 原生 SFTP 下载完成 (双缓冲流水线): {} -> {} ({} bytes)",
            remote_path, local_path, transferred
        );
        Ok(())
    }

    async fn upload_file(
        &self,
        session_id: &str,
        local_path: &str,
        remote_path: &str,
        progress_tx: Option<UnboundedSender<TransferProgress>>,
    ) -> SshServiceResult<()> {
        let sftp = self.get_session(session_id).await?;
        let mut local_file = tokio::fs::File::open(local_path)
            .await
            .map_err(SshServiceError::Io)?;

        let total_size = local_file.metadata().await.map(|m| m.len()).unwrap_or(0);

        let mut remote_file = sftp
            .create(remote_path)
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("创建远程文件失败: {}", e)))?;

        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let chunk_size = 128 * 1024; // 128 KB 分块
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);

        let start_time = std::time::Instant::now();
        let reader_fut = async {
            let mut read_buf = vec![0u8; chunk_size];
            loop {
                let bytes_read = local_file.read(&mut read_buf).await?;
                if bytes_read == 0 {
                    break;
                }
                if tx.send(read_buf[..bytes_read].to_vec()).await.is_err() {
                    break;
                }
            }
            drop(tx);
            Ok::<(), std::io::Error>(())
        };

        let writer_fut = async {
            let mut transferred = 0u64;
            let mut last_progress_send = std::time::Instant::now();
            let mut last_sample_time = std::time::Instant::now();
            let mut last_sample_bytes = 0u64;
            let mut current_speed = 0u64;

            while let Some(chunk) = rx.recv().await {
                remote_file.write_all(&chunk).await?;
                transferred += chunk.len() as u64;

                if let Some(ref tx_prog) = progress_tx {
                    let now = std::time::Instant::now();
                    let sample_elapsed = now.duration_since(last_sample_time).as_secs_f64();
                    if sample_elapsed >= 0.2 {
                        let bytes_in_sample = transferred.saturating_sub(last_sample_bytes);
                        let instant_speed = (bytes_in_sample as f64 / sample_elapsed) as u64;
                        current_speed = if current_speed == 0 {
                            instant_speed
                        } else {
                            ((instant_speed as f64 * 0.7) + (current_speed as f64 * 0.3)) as u64
                        };
                        last_sample_time = now;
                        last_sample_bytes = transferred;
                    }

                    if now.duration_since(last_progress_send).as_millis() >= 100 {
                        let _ = tx_prog.send(TransferProgress {
                            task_id: format!("ul-{}", local_path),
                            transferred_bytes: transferred,
                            total_bytes: total_size,
                            speed_bytes_per_sec: current_speed,
                        });
                        last_progress_send = now;
                    }
                }
            }

            remote_file.flush().await?;
            Ok::<u64, std::io::Error>(transferred)
        };

        let (_, transferred) = match tokio::try_join!(reader_fut, writer_fut) {
            Ok(res) => res,
            Err(e) => {
                let _ = sftp.remove_file(remote_path).await;
                return Err(SshServiceError::Io(e));
            }
        };

        if let Some(ref tx) = progress_tx {
            let elapsed_secs = start_time.elapsed().as_secs_f64().max(0.001);
            let avg_speed = (transferred as f64 / elapsed_secs) as u64;
            let _ = tx.send(TransferProgress {
                task_id: format!("ul-{}", local_path),
                transferred_bytes: transferred,
                total_bytes: total_size,
                speed_bytes_per_sec: avg_speed,
            });
        }

        info!(
            target: "smagical_ssh::sftp",
            "纯 Rust 原生 SFTP 上传完成 (双缓冲流水线): {} -> {} ({} bytes)",
            local_path, remote_path, transferred
        );
        Ok(())
    }
}

/// 格式化 Unix 文件权限八进制位为 rwxrwxrwx
fn format_permissions(permissions: Option<u32>) -> String {
    let mode = permissions.unwrap_or(0o644);
    let r = |bit: u32, ch: char| if mode & bit != 0 { ch } else { '-' };
    format!(
        "{}{}{}{}{}{}{}{}{}",
        r(0o400, 'r'),
        r(0o200, 'w'),
        r(0o100, 'x'),
        r(0o040, 'r'),
        r(0o020, 'w'),
        r(0o010, 'x'),
        r(0o004, 'r'),
        r(0o002, 'w'),
        r(0o001, 'x'),
    )
}

/// 将 Unix Epoch 秒时间戳格式化为本地易读时间
fn format_epoch_seconds(epoch: u64) -> String {
    if epoch == 0 {
        return "-".to_string();
    }
    chrono::DateTime::from_timestamp(epoch as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_permissions() {
        assert_eq!(format_permissions(Some(0o755)), "rwxr-xr-x");
        assert_eq!(format_permissions(Some(0o644)), "rw-r--r--");
        assert_eq!(format_permissions(Some(0o700)), "rwx------");
        assert_eq!(format_permissions(Some(0o777)), "rwxrwxrwx");
        assert_eq!(format_permissions(None), "rw-r--r--");
    }

    #[test]
    fn test_format_epoch_seconds() {
        assert_eq!(format_epoch_seconds(0), "-");
        let formatted = format_epoch_seconds(1700000000);
        assert!(formatted.contains("2023"));
    }

    #[tokio::test]
    async fn test_unregistered_session_returns_not_found() {
        let driver = RusshSftpDriver::new();
        let res = driver.list_dir("non_existent_sess", "/").await;
        assert!(res.is_err());
        match res.unwrap_err() {
            SshServiceError::NotFound(msg) => {
                assert!(msg.contains("non_existent_sess"));
            }
            other => panic!("预期返回 NotFound，实际返回: {:?}", other),
        }
    }
}
