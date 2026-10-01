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
        self.connect_raw(
            session_id,
            &host.address,
            host.port,
            username,
            password,
            private_key_pem,
        ).await
    }

    /// 使用 SshLaunchConfig 快速建立纯 Rust 原生 SFTP 会话并注入会话池 (用于连接复用)
    pub async fn connect_and_register_with_config(
        &self,
        session_id: &str,
        config: &crate::ssh_config::SshLaunchConfig,
    ) -> SshServiceResult<()> {
        let username = config.username.as_deref().unwrap_or("root");
        self.connect_raw(
            session_id,
            &config.host,
            config.port,
            username,
            config.password.as_deref(),
            config.private_key_pem.as_deref(),
        ).await
    }

    /// 统一底层 SFTP 连接与鉴权实现
    async fn connect_raw(
        &self,
        session_id: &str,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        private_key_pem: Option<&str>,
    ) -> SshServiceResult<()> {
        info!(
            target: "smagical_ssh::sftp",
            "正在发起原生纯 Rust SFTP 连接握手: {}@{}:{}",
            username, host, port
        );

        let config = Arc::new(russh::client::Config::default());
        let addr = format!("{}:{}", host, port);
        let mut handler = SftpClientHandler::new(host, port, "accept_new");
        handler.known_hosts_path = self.known_hosts_path.clone();

        let mut handle = russh::client::connect(config, addr.as_str(), handler)
            .await
            .map_err(|e| SshServiceError::HostUnreachable(format!("连接远程主机失败: {}", e)))?;

        // 凭据认证优先级：私钥优先，若无或失败则密码认证
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
            return Err(SshServiceError::AuthFailed("用户名或密码/私钥认证失败".to_string()));
        }

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

        let mut buffer = vec![0u8; 1024 * 64]; // 64 KB 分块流水线
        let mut transferred = 0u64;
        let start_time = std::time::Instant::now();
        let mut last_progress_send = std::time::Instant::now();
        let mut last_sample_time = std::time::Instant::now();
        let mut last_sample_bytes = 0u64;
        let mut current_speed = 0u64;

        loop {
            let bytes_read = remote_file
                .read(&mut buffer)
                .await
                .map_err(SshServiceError::Io)?;

            if bytes_read == 0 {
                break;
            }

            local_file
                .write_all(&buffer[..bytes_read])
                .await
                .map_err(SshServiceError::Io)?;

            transferred += bytes_read as u64;

            if let Some(ref tx) = progress_tx {
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
                    let _ = tx.send(TransferProgress {
                        task_id: format!("dl-{}", remote_path),
                        transferred_bytes: transferred,
                        total_bytes: total_size,
                        speed_bytes_per_sec: current_speed,
                    });
                    last_progress_send = now;
                }
            }
        }

        local_file.flush().await.map_err(SshServiceError::Io)?;

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
            "纯 Rust 原生 SFTP 下载完成: {} -> {} ({} bytes)",
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

        let mut buffer = vec![0u8; 1024 * 64]; // 64 KB 分块
        let mut transferred = 0u64;
        let start_time = std::time::Instant::now();
        let mut last_progress_send = std::time::Instant::now();
        let mut last_sample_time = std::time::Instant::now();
        let mut last_sample_bytes = 0u64;
        let mut current_speed = 0u64;

        loop {
            let bytes_read = local_file
                .read(&mut buffer)
                .await
                .map_err(SshServiceError::Io)?;

            if bytes_read == 0 {
                break;
            }

            remote_file
                .write_all(&buffer[..bytes_read])
                .await
                .map_err(SshServiceError::Io)?;

            transferred += bytes_read as u64;

            if let Some(ref tx) = progress_tx {
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
                    let _ = tx.send(TransferProgress {
                        task_id: format!("ul-{}", local_path),
                        transferred_bytes: transferred,
                        total_bytes: total_size,
                        speed_bytes_per_sec: current_speed,
                    });
                    last_progress_send = now;
                }
            }
        }

        remote_file.flush().await.map_err(SshServiceError::Io)?;

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
            "纯 Rust 原生 SFTP 上传完成: {} -> {} ({} bytes)",
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
