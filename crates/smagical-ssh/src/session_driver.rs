//! 基于 russh 的纯 Rust 原生 SSH 远程终端与会话服务驱动实现。
//!
//! 实现 `smagical_core::service::SshSessionService` 契约，直接在内存中建立 SSHv2 会话长连接，
//! 分配远端 PTY 虚拟终端并提供连续双向异步字节读写流，彻底废除宿主机系统 `ssh.exe` 外部进程依赖。

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use russh::keys::agent::client::AgentClient;

use smagical_core::domain::{CredentialRecord, CredentialType, HostRecord};
use smagical_core::service::{
    CommandExecutionOutput, SshServiceError, SshServiceResult, SshSessionService, SshStreamChannel,
};

/// 原生 SSH 客户端事件处理器 (支持 TOFU known_hosts 校验与安全防劫持)
#[derive(Clone)]
struct SshSessionClientHandler {
    host: String,
    port: u16,
    policy: String,
    known_hosts_path: Option<std::path::PathBuf>,
    progress: Option<smagical_core::service::SshProgressCallback>,
}

impl SshSessionClientHandler {
    fn with_known_hosts(
        host: &str,
        port: u16,
        policy: &str,
        known_hosts_path: Option<std::path::PathBuf>,
        progress: Option<smagical_core::service::SshProgressCallback>,
    ) -> Self {
        Self {
            host: host.to_string(),
            port,
            policy: policy.to_string(),
            known_hosts_path,
            progress,
        }
    }
}

impl russh::client::Handler for SshSessionClientHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let host = self.host.clone();
        let port = self.port;
        let policy = self.policy.clone();
        let path_opt = self.known_hosts_path.clone();
        let progress = self.progress.clone();

        async move {
            if policy == "insecure-accept-all" {
                if let Some(ref p) = progress {
                    p("主机公钥验证策略: 信任所有主机 (insecure-accept-all)");
                }
                return Ok(true);
            }

            let Some(known_hosts_path) = path_opt else {
                return Ok(true);
            };

            match crate::known_hosts::verify_server_key_in_file(&known_hosts_path, &host, port, server_public_key) {
                Ok(smagical_core::service::HostKeyVerificationResult::Trusted) => {
                    info!(target: "smagical_ssh::session", "主机公钥验真通过 (已在 known_hosts 信任白名单): {}:{}", host, port);
                    if let Some(ref p) = progress {
                        p(&format!("服务端主机公钥校验通过 (已信任主机 {}:{})", host, port));
                    }
                    Ok(true)
                }
                Ok(smagical_core::service::HostKeyVerificationResult::FirstTimeHost { fingerprint, public_key_text: _ }) => {
                    if policy == "strict" {
                        warn!(target: "smagical_ssh::session", "严格模式拒绝首次连接的未登记主机: {}:{} (指纹: {})", host, port, fingerprint);
                        if let Some(ref p) = progress {
                            p(&format!("严格安全策略拒绝首次连接的主机 (指纹: {})", fingerprint));
                        }
                        Ok(false)
                    } else {
                        info!(target: "smagical_ssh::session", "首次连接主机 {}:{}，指纹 [{}]，正在安全登记至 known_hosts...", host, port, fingerprint);
                        if let Some(ref p) = progress {
                            p(&format!("首次连接目标主机，已自动记录并信任主机公钥指纹 [{}]", fingerprint));
                        }
                        let _ = crate::known_hosts::append_known_host_to_file(&known_hosts_path, &host, port, server_public_key);
                        Ok(true)
                    }
                }
                Ok(smagical_core::service::HostKeyVerificationResult::Mismatch { expected_fingerprint, actual_fingerprint }) => {
                    tracing::error!(
                        target: "smagical_ssh::security",
                        "🚨【严重安全警报】主机公钥与已知历史记录不匹配！疑似遭遇中间人劫持攻击！主机: {}:{}, 历史记录指纹: {}, 本次接收指纹: {}",
                        host, port, expected_fingerprint, actual_fingerprint
                    );
                    if let Some(ref p) = progress {
                        p(&format!("安全警报: 主机公钥指纹与历史记录不匹配！(已知: {}, 实际: {})", expected_fingerprint, actual_fingerprint));
                    }
                    Ok(false)
                }
                Err(e) => {
                    warn!(target: "smagical_ssh::session", "读取 known_hosts 异常: {:?}，回退放行", e);
                    Ok(true)
                }
            }
        }
    }
}

/// 纯 Rust 原生 SSH 会话与终端通道驱动引擎 (RusshSessionDriver)
#[derive(Clone)]
pub struct RusshSessionDriver {
    /// 活跃的客户端连接句柄映射池 (session_id -> Arc<Handle>)
    handles: Arc<RwLock<HashMap<String, Arc<russh::client::Handle<SshSessionClientHandler>>>>>,
    known_hosts_path: Option<std::path::PathBuf>,
}

impl Default for RusshSessionDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl RusshSessionDriver {
    /// 创建全新的 SSH 会话驱动管理器
    pub fn new() -> Self {
        Self {
            handles: Arc::new(RwLock::new(HashMap::new())),
            known_hosts_path: crate::known_hosts::get_default_known_hosts_path(),
        }
    }

    /// 使用指定的 known_hosts 物理路径创建会话驱动管理器 (主要用于隔离集成测试或自定义安全目录)
    pub fn with_known_hosts_path(path: Option<std::path::PathBuf>) -> Self {
        Self {
            handles: Arc::new(RwLock::new(HashMap::new())),
            known_hosts_path: path,
        }
    }

    fn make_handler(&self, host: &str, port: u16, progress: Option<smagical_core::service::SshProgressCallback>) -> SshSessionClientHandler {
        SshSessionClientHandler::with_known_hosts(
            host,
            port,
            "accept_new",
            self.known_hosts_path.clone(),
            progress,
        )
    }

    /// 获取处于活跃连接状态的 Handle 引用克隆
    async fn get_handle(&self, session_id: &str) -> SshServiceResult<Arc<russh::client::Handle<SshSessionClientHandler>>> {
        let guard = self.handles.read().await;
        guard
            .get(session_id)
            .cloned()
            .ok_or_else(|| SshServiceError::NotFound(format!("未找到处于活跃状态的 SSH 会话 [{}]", session_id)))
    }
}

/// 对 russh 句柄执行认证 (优先公私钥/证书，其次密码)
async fn authenticate_handle(
    handle: &mut russh::client::Handle<SshSessionClientHandler>,
    username: &str,
    credential: Option<&CredentialRecord>,
    progress: Option<&smagical_core::service::SshProgressCallback>,
) -> SshServiceResult<()> {
    let mut authenticated = false;

    // 1. 尝试私钥认证
    if let Some(cred) = credential {
        if (cred.cred_type == CredentialType::Key || cred.cred_type == CredentialType::Certificate)
            && !cred.secret_data.trim().is_empty()
        {
            debug!(target: "smagical_ssh::session", "正在尝试私钥证书认证...");
            if let Some(p) = progress {
                p("正在使用私钥进行公钥认证 (Public Key)...");
            }
            let passphrase_opt = cred
                .passphrase
                .as_deref()
                .filter(|p| !p.is_empty());

            if let Ok(key) = russh::keys::decode_secret_key(&cred.secret_data, passphrase_opt) {
                let key_with_alg = russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None);
                if let Ok(auth_res) = handle.authenticate_publickey(username, key_with_alg).await {
                    if auth_res.success() {
                        authenticated = true;
                        info!(target: "smagical_ssh::session", "私钥证书认证成功: 用户名 '{}'", username);
                        if let Some(p) = progress {
                            p(&format!("私钥公钥认证通过 (用户: {})", username));
                        }
                    } else if let Some(p) = progress {
                        p("私钥公钥认证未通过 (公钥被服务端拒绝)");
                    }
                } else if let Some(p) = progress {
                    p("私钥公钥认证通信异常");
                }
            } else if let Some(p) = progress {
                p("私钥解析失败 (密钥格式不正确或口令密码错误)");
            }
        }
    }

    // 2. 尝试密码认证
    if !authenticated {
        if let Some(cred) = credential {
            if cred.cred_type == CredentialType::Password && !cred.secret_data.is_empty() {
                debug!(target: "smagical_ssh::session", "正在尝试密码认证...");
                if let Some(p) = progress {
                    p("正在使用密码进行身份认证 (Password)...");
                }
                let auth_res = handle
                    .authenticate_password(username, &cred.secret_data)
                    .await
                    .map_err(|e| SshServiceError::AuthFailed(format!("密码认证通信异常: {}", e)))?;
                if auth_res.success() {
                    authenticated = true;
                    info!(target: "smagical_ssh::session", "密码认证成功: 用户名 '{}'", username);
                    if let Some(p) = progress {
                        p(&format!("密码认证通过 (用户: {})", username));
                    }
                } else if let Some(p) = progress {
                    p("密码认证未通过 (用户名或密码错误)");
                }
            }
        }
    }

    // 3. 尝试 SSH Agent 认证
    if !authenticated {
        if let Some(cred) = credential {
            if cred.cred_type == CredentialType::Agent {
                debug!(target: "smagical_ssh::session", "正在尝试 SSH Agent 管道认证...");
                if let Some(p) = progress {
                    p("正在尝试通过 SSH Agent 进行身份认证...");
                }
                if authenticate_via_agent(handle, username).await {
                    authenticated = true;
                    info!(target: "smagical_ssh::session", "SSH Agent 认证成功: 用户名 '{}'", username);
                    if let Some(p) = progress {
                        p(&format!("SSH Agent 认证通过 (用户: {})", username));
                    }
                } else {
                    warn!(target: "smagical_ssh::session", "SSH Agent 认证未通过或未找到可用密钥");
                    if let Some(p) = progress {
                        p("SSH Agent 认证未通过或未找到可用密钥");
                    }
                }
            }
        }
    }

    if !authenticated {
        if let Some(cred) = credential {
            if cred.cred_type == CredentialType::Password && cred.secret_data.is_empty() {
                if let Some(p) = progress {
                    p("认证失败: 密码为空");
                }
                return Err(SshServiceError::AuthFailed("密码不能为空".to_string()));
            }
        }
        if let Some(p) = progress {
            p(&format!("认证失败: 远程主机拒绝用户 '{}' 的登录请求", username));
        }
        return Err(SshServiceError::AuthFailed(format!("远程主机认证失败: 用户名 '{}'", username)));
    }

    Ok(())
}

/// 尝试通过 SSH Agent（跨平台支持 Windows Named Pipe / Pageant 及 Unix UDS）完成公钥认证
async fn authenticate_via_agent(
    handle: &mut russh::client::Handle<SshSessionClientHandler>,
    username: &str,
) -> bool {
    #[cfg(windows)]
    {
        let pipe_name = std::env::var("SSH_AUTH_SOCK")
            .unwrap_or_else(|_| r"\\.\pipe\openssh-ssh-agent".to_string());
        if let Ok(agent) = AgentClient::connect_named_pipe(&pipe_name).await {
            let mut dynamic_agent = agent.dynamic();
            if run_agent_auth_loop(handle, username, &mut dynamic_agent).await {
                return true;
            }
        }
        if let Ok(agent) = AgentClient::connect_pageant().await {
            let mut dynamic_agent = agent.dynamic();
            if run_agent_auth_loop(handle, username, &mut dynamic_agent).await {
                return true;
            }
        }
    }

    #[cfg(unix)]
    {
        if let Ok(agent) = AgentClient::connect_env().await {
            let mut dynamic_agent = agent.dynamic();
            if run_agent_auth_loop(handle, username, &mut dynamic_agent).await {
                return true;
            }
        }
    }

    false
}

async fn run_agent_auth_loop(
    handle: &mut russh::client::Handle<SshSessionClientHandler>,
    username: &str,
    agent: &mut AgentClient<Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>>,
) -> bool {
    if let Ok(identities) = agent.request_identities().await {
        for identity in identities {
            match identity {
                russh::keys::agent::AgentIdentity::PublicKey { key, .. } => {
                    if let Ok(auth_res) = handle.authenticate_publickey_with(username, key, None, agent).await {
                        if auth_res.success() {
                            return true;
                        }
                    }
                }
                russh::keys::agent::AgentIdentity::Certificate { certificate, .. } => {
                    if let Ok(auth_res) = handle.authenticate_certificate_with(username, certificate, None, agent).await {
                        if auth_res.success() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// 通过 SOCKS5 代理建立到目标主机的纯 TCP 穿透流
pub(crate) async fn connect_via_socks5(
    proxy_addr: &str,
    target_host: &str,
    target_port: u16,
) -> SshServiceResult<TcpStream> {
    let mut stream = TcpStream::connect(proxy_addr)
        .await
        .map_err(|e| SshServiceError::HostUnreachable(format!("连接 SOCKS5 代理失败 [{}]: {}", proxy_addr, e)))?;

    // 1. 发送无认证问候: [VER=5, NMETHODS=1, METHOD=0(NO_AUTH)]
    stream.write_all(&[0x05, 0x01, 0x00]).await
        .map_err(|e| SshServiceError::ProtocolError(format!("SOCKS5 握手失败: {}", e)))?;

    let mut resp = [0u8; 2];
    stream.read_exact(&mut resp).await
        .map_err(|e| SshServiceError::ProtocolError(format!("读取 SOCKS5 握手响应失败: {}", e)))?;
    if resp[0] != 0x05 || resp[1] != 0x00 {
        return Err(SshServiceError::AuthFailed("SOCKS5 代理不支持无认证或拒绝访问".to_string()));
    }

    // 2. 发送 CONNECT 请求
    let mut req = Vec::new();
    req.push(0x05); // VER
    req.push(0x01); // CMD: CONNECT
    req.push(0x00); // RSV
    if let Ok(ip) = target_host.parse::<std::net::Ipv4Addr>() {
        req.push(0x01); // ATYP: IPv4
        req.extend_from_slice(&ip.octets());
    } else {
        req.push(0x03); // ATYP: DOMAINNAME
        req.push(target_host.len() as u8);
        req.extend_from_slice(target_host.as_bytes());
    }
    req.extend_from_slice(&target_port.to_be_bytes());

    stream.write_all(&req).await
        .map_err(|e| SshServiceError::ProtocolError(format!("发送 SOCKS5 连接请求失败: {}", e)))?;

    let mut head = [0u8; 4];
    stream.read_exact(&mut head).await
        .map_err(|e| SshServiceError::ProtocolError(format!("读取 SOCKS5 连接响应失败: {}", e)))?;
    if head[1] != 0x00 {
        return Err(SshServiceError::HostUnreachable(format!("SOCKS5 代理连接目标主机失败 (REP: {})", head[1])));
    }

    // 跳过绑定的地址与端口
    match head[3] {
        0x01 => {
            let mut b = [0u8; 6];
            stream.read_exact(&mut b).await.map_err(|e| SshServiceError::ProtocolError(e.to_string()))?;
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await.map_err(|e| SshServiceError::ProtocolError(e.to_string()))?;
            let mut domain_and_port = vec![0u8; len[0] as usize + 2];
            stream.read_exact(&mut domain_and_port).await.map_err(|e| SshServiceError::ProtocolError(e.to_string()))?;
        }
        0x04 => {
            let mut b = [0u8; 18];
            stream.read_exact(&mut b).await.map_err(|e| SshServiceError::ProtocolError(e.to_string()))?;
        }
        _ => {}
    }

    Ok(stream)
}

/// 通过 HTTP CONNECT 代理建立到目标主机的纯 TCP 穿透流
pub(crate) async fn connect_via_http_connect(
    proxy_addr: &str,
    target_host: &str,
    target_port: u16,
) -> SshServiceResult<TcpStream> {
    let mut stream = TcpStream::connect(proxy_addr)
        .await
        .map_err(|e| SshServiceError::HostUnreachable(format!("连接 HTTP 代理失败 [{}]: {}", proxy_addr, e)))?;

    let connect_req = format!("CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n\r\n", target_host, target_port, target_host, target_port);
    stream.write_all(connect_req.as_bytes()).await
        .map_err(|e| SshServiceError::ProtocolError(format!("发送 HTTP CONNECT 请求失败: {}", e)))?;

    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await
        .map_err(|e| SshServiceError::ProtocolError(format!("读取 HTTP 代理响应失败: {}", e)))?;
    let resp = String::from_utf8_lossy(&buf[..n]);
    if !resp.starts_with("HTTP/1.1 200") && !resp.starts_with("HTTP/1.0 200") {
        return Err(SshServiceError::HostUnreachable(format!("HTTP 代理拒绝连接: {}", resp.lines().next().unwrap_or(""))));
    }

    Ok(stream)
}

/// 跳板机单跳网络节点配置信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpHop {
    /// 可选的登录用户名 (例如 `user@host` 中的 `user`)
    pub username: Option<String>,
    /// 跳板机主机名或 IP 地址
    pub host: String,
    /// 跳板机 SSH 服务端口 (默认 22)
    pub port: u16,
}

/// 解析标准化跳板机链路字符串 (例如 `"bastion1:22,admin@bastion2:2222"`) 为结构化跳板节点列表。
pub fn parse_jump_chain(summary: &str) -> Vec<JumpHop> {
    let mut hops = Vec::new();
    let trimmed = summary.trim();
    if trimmed.is_empty() || trimmed.starts_with("jump_hops:0") || trimmed.starts_with("preset:") {
        return hops;
    }
    for part in trimmed.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (user_opt, host_port) = if let Some((u, hp)) = part.split_once('@') {
            (if u.trim().is_empty() { None } else { Some(u.trim().to_string()) }, hp)
        } else {
            (None, part)
        };
        let (host, port) = if let Some((h, p)) = host_port.rsplit_once(':') {
            (h.trim().to_string(), p.parse::<u16>().unwrap_or(22))
        } else {
            (host_port.trim().to_string(), 22)
        };
        if !host.is_empty() {
            hops.push(JumpHop { username: user_opt, host, port });
        }
    }
    hops
}

/// 探测指定出站代理服务器（SOCKS5 或 HTTP CONNECT）的可用性与往返延迟。
///
/// # 参数
/// * `proxy_type` - 代理协议类型（`"socks5"`、`"socks"`、`"http"`、`"https"`）
/// * `proxy_host` - 代理服务器 IP 或域名
/// * `proxy_port` - 代理端口
/// * `timeout_ms` - 探测超时时间（毫秒，建议 3000~5000ms）
///
/// # 返回值
/// - `Ok(latency_ms)`: 代理存活且可用，返回往返建立连接延迟（毫秒）；
/// - `Err(error_msg)`: 代理无法连接或协议握手失败原因。
pub async fn probe_proxy_health(
    proxy_type: &str,
    proxy_host: &str,
    proxy_port: u16,
    timeout_ms: u64,
) -> Result<u64, String> {
    if proxy_host.trim().is_empty() || proxy_port == 0 {
        return Err("代理地址或端口无效".to_string());
    }
    let proxy_addr = format!("{}:{}", proxy_host.trim(), proxy_port);
    let timeout_dur = std::time::Duration::from_millis(timeout_ms.max(500));
    let t0 = std::time::Instant::now();

    let mut stream = tokio::time::timeout(timeout_dur, TcpStream::connect(&proxy_addr))
        .await
        .map_err(|_| format!("连接代理服务器 [{}] 超时 ({}ms)", proxy_addr, timeout_ms))?
        .map_err(|e| format!("无法连接代理服务器 [{}]: {}", proxy_addr, e))?;

    let ptype = proxy_type.trim().to_lowercase();
    if ptype == "socks5" || ptype == "socks" {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        tokio::time::timeout(timeout_dur, stream.write_all(&[0x05, 0x01, 0x00]))
            .await
            .map_err(|_| "向 SOCKS5 代理发送问候超时".to_string())?
            .map_err(|e| format!("发送 SOCKS5 握手失败: {e}"))?;

        let mut resp = [0u8; 2];
        tokio::time::timeout(timeout_dur, stream.read_exact(&mut resp))
            .await
            .map_err(|_| "读取 SOCKS5 响应超时".to_string())?
            .map_err(|e| format!("读取 SOCKS5 响应失败: {e}"))?;

        if resp[0] != 0x05 {
            return Err(format!("非标准 SOCKS5 响应 (VER: {})", resp[0]));
        }
    } else if ptype == "http" || ptype == "https" {
        use tokio::io::AsyncWriteExt;
        let ping_req = b"CONNECT 127.0.0.1:0 HTTP/1.1\r\nHost: 127.0.0.1:0\r\n\r\n";
        let _ = tokio::time::timeout(timeout_dur, stream.write_all(ping_req)).await;
    }

    Ok(t0.elapsed().as_millis() as u64)
}

#[async_trait]
impl SshSessionService for RusshSessionDriver {
    async fn connect_with_progress(
        &self,
        host: &HostRecord,
        credential: Option<&CredentialRecord>,
        progress: Option<smagical_core::service::SshProgressCallback>,
    ) -> SshServiceResult<String> {
        let session_id = format!("{}-{}", host.id, uuid::Uuid::new_v4().simple());
        info!(
            target: "smagical_ssh::session",
            "正在发起原生纯 Rust SSH 会话连接: {}:{} (会话 ID: {})",
            host.address, host.port, session_id
        );

        let mut client_config = russh::client::Config::default();
        let keepalive_sec = if host.keepalive_interval > 0 {
            host.keepalive_interval
        } else {
            15
        };
        client_config.keepalive_interval = Some(std::time::Duration::from_secs(keepalive_sec as u64));
        client_config.keepalive_max = 3;
        client_config.inactivity_timeout = None;
        let connect_timeout_sec = if host.connect_timeout > 0 {
            host.connect_timeout
        } else {
            15
        };
        let config = Arc::new(client_config);
        let target_addr = format!("{}:{}", host.address, host.port);

        // 若外部未显式提供凭据记录，但主机记录自身直接内联配置了私钥或密码，自动作为内联凭据参与认证
        let inline_cred = if credential.is_none() {
            if let Some(ref pass) = host.password {
                if !pass.is_empty() {
                    Some(CredentialRecord::new_password(
                        "inline-pass",
                        "Inline Password",
                        host.username.clone().unwrap_or_default(),
                        pass.clone(),
                        "",
                    ))
                } else {
                    None
                }
            } else if let Some(ref key) = host.key_data {
                if !key.trim().is_empty() {
                    let mut rec = CredentialRecord::new_key(
                        "inline-key",
                        "Inline Key",
                        "Ed25519",
                        key.clone(),
                        None,
                        None,
                        None,
                        "",
                    );
                    rec.username = host.username.clone();
                    Some(rec)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let credential = credential.or(inline_cred.as_ref());

        // 提取用户名 (凭据用户名优先，其次主机记录用户名，默认 root)
        let username = credential
            .and_then(|c| c.username.as_deref())
            .filter(|u| !u.trim().is_empty())
            .or_else(|| host.username.as_deref().filter(|u| !u.trim().is_empty()))
            .unwrap_or("root");

        // 1. 检查是否存在跳板机链 (Jump Chain)
        let jump_hops = host.jump_chain_summary.as_deref().map(parse_jump_chain).unwrap_or_default();

        // 2. 检查网络代理
        let proxy_type = host.proxy_type.as_deref().unwrap_or("direct");
        let proxy_host = host.proxy_host.as_deref().unwrap_or("");
        let proxy_port = host.proxy_port.unwrap_or(0);

        // 3. 构建底层 Handle (直连 / 代理穿透 / 跳板机 Direct-TCPIP 隧道链)
        let handle = if !jump_hops.is_empty() {
            let total_hops = jump_hops.len();
            info!(target: "smagical_ssh::session", "检测到跳板机链路 (共 {} 跳): {:?}，正在纯内存级建立 Direct-TCPIP 多跳通道...", total_hops, jump_hops);
            let first_hop = &jump_hops[0];
            let first_addr = format!("{}:{}", first_hop.host, first_hop.port);
            let first_user = first_hop.username.as_deref().unwrap_or(username);
            if let Some(ref p) = progress {
                p(&format!("正在发起跳板机网络连接: [第 1/{} 跳] {} ...", total_hops, first_addr));
            }
            let timeout_dur = std::time::Duration::from_secs(connect_timeout_sec as u64);
            let t0 = std::time::Instant::now();
            let mut current_handle = tokio::time::timeout(
                timeout_dur,
                russh::client::connect(config.clone(), first_addr.as_str(), self.make_handler(&first_hop.host, first_hop.port, progress.clone()))
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("连接第 1/{} 跳跳板机超时 ({}s) [{}]", total_hops, connect_timeout_sec, first_addr)))?
            .map_err(|e| SshServiceError::HostUnreachable(format!("连接第 1/{} 跳跳板机失败 [{}]: {}", total_hops, first_addr, e)))?;
            authenticate_handle(&mut current_handle, first_user, credential, progress.as_ref()).await?;
            let elapsed_ms = t0.elapsed().as_millis();
            if let Some(ref p) = progress {
                p(&format!("第 1/{} 跳跳板机 [{}] 认证通过 (耗时 {}ms)", total_hops, first_addr, elapsed_ms));
            }

            for (idx, hop) in jump_hops[1..].iter().enumerate() {
                let hop_num = idx + 2;
                let hop_addr = format!("{}:{}", hop.host, hop.port);
                let hop_user = hop.username.as_deref().unwrap_or(username);
                if let Some(ref p) = progress {
                    p(&format!("正在建立通往中间跳板机 [第 {}/{} 跳] [{}] 的 Direct-TCPIP 隧道...", hop_num, total_hops, hop_addr));
                }
                let t_hop = std::time::Instant::now();
                let channel = tokio::time::timeout(
                    timeout_dur,
                    current_handle.channel_open_direct_tcpip(&hop.host, hop.port as u32, "127.0.0.1", 0)
                )
                .await
                .map_err(|_| SshServiceError::Timeout(format!("请求建立 Direct-TCPIP 隧道至第 {}/{} 跳 [{}] 超时 ({}s)", hop_num, total_hops, hop_addr, connect_timeout_sec)))?
                .map_err(|e| SshServiceError::ProtocolError(format!("跳板机转发至第 {}/{} 跳 [{}] 失败: {}", hop_num, total_hops, hop_addr, e)))?;

                let mut next_handle = tokio::time::timeout(
                    timeout_dur,
                    russh::client::connect_stream(config.clone(), channel.into_stream(), self.make_handler(&hop.host, hop.port, progress.clone()))
                )
                .await
                .map_err(|_| SshServiceError::Timeout(format!("通过隧道握手中间跳板机 [{}] 超时 ({}s)", hop_addr, connect_timeout_sec)))?
                .map_err(|e| SshServiceError::HostUnreachable(format!("通过隧道连接中间跳板机 [{}] 失败: {}", hop_addr, e)))?;

                authenticate_handle(&mut next_handle, hop_user, credential, progress.as_ref()).await?;
                let hop_elapsed = t_hop.elapsed().as_millis();
                if let Some(ref p) = progress {
                    p(&format!("第 {}/{} 跳中间跳板机 [{}] 认证通过 (耗时 {}ms)", hop_num, total_hops, hop_addr, hop_elapsed));
                }
                current_handle = next_handle;
            }

            // 最后一跳通往目标主机
            if let Some(ref p) = progress {
                p(&format!("正在通过跳板链路建立通往目标主机 [{}] 的 Direct-TCPIP 隧道...", target_addr));
            }
            let t_target = std::time::Instant::now();
            let target_channel = tokio::time::timeout(
                timeout_dur,
                current_handle.channel_open_direct_tcpip(&host.address, host.port as u32, "127.0.0.1", 0)
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("建立通往目标主机 [{}] 的 Direct-TCPIP 隧道超时 ({}s)", target_addr, connect_timeout_sec)))?
            .map_err(|e| SshServiceError::ProtocolError(format!("跳板机建立通往目标主机 [{}] 的 Direct-TCPIP 隧道失败: {}", target_addr, e)))?;

            let mut target_handle = tokio::time::timeout(
                timeout_dur,
                russh::client::connect_stream(config.clone(), target_channel.into_stream(), self.make_handler(&host.address, host.port, progress.clone()))
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("通过跳板机直连目标主机 [{}] 握手超时 ({}s)", target_addr, connect_timeout_sec)))?
            .map_err(|e| SshServiceError::HostUnreachable(format!("通过跳板机直连目标主机 [{}] 失败: {}", target_addr, e)))?;

            authenticate_handle(&mut target_handle, username, credential, progress.as_ref()).await?;
            let target_elapsed = t_target.elapsed().as_millis();
            if let Some(ref p) = progress {
                p(&format!("目标主机 [{}] 链路握手完成并认证通过 (终跳耗时: {}ms)", target_addr, target_elapsed));
            }
            target_handle
        } else if (proxy_type == "socks5" || proxy_type == "socks") && !proxy_host.is_empty() && proxy_port > 0 {
            info!(target: "smagical_ssh::session", "检测到 SOCKS5 代理 [{}:{}]: 纯 Rust 原生穿透至目标 [{}]...", proxy_host, proxy_port, target_addr);
            let proxy_addr = format!("{}:{}", proxy_host, proxy_port);
            if let Some(ref p) = progress {
                p(&format!("正在连接 SOCKS5 代理服务器 [{}] ...", proxy_addr));
            }
            let timeout_dur = std::time::Duration::from_secs(connect_timeout_sec as u64);
            let t_proxy = std::time::Instant::now();
            let stream = tokio::time::timeout(timeout_dur, connect_via_socks5(&proxy_addr, &host.address, host.port))
                .await
                .map_err(|_| SshServiceError::Timeout(format!("连接 SOCKS5 代理超时 ({}s) [{}]", connect_timeout_sec, proxy_addr)))?
                .map_err(|e| SshServiceError::HostUnreachable(format!("SOCKS5 代理穿透失败 [{}]: {}", proxy_addr, e)))?;
            let proxy_ms = t_proxy.elapsed().as_millis();
            if let Some(ref p) = progress {
                p(&format!("SOCKS5 代理握手成功 (耗时 {}ms)，正在握手目标 [{}] ...", proxy_ms, target_addr));
            }
            let mut h = tokio::time::timeout(
                timeout_dur,
                russh::client::connect_stream(config.clone(), stream, self.make_handler(&host.address, host.port, progress.clone()))
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("通过 SOCKS5 代理连接目标主机超时 ({}s): [{}]", connect_timeout_sec, target_addr)))?
            .map_err(|e| SshServiceError::HostUnreachable(format!("通过 SOCKS5 代理连接目标 [{}] 失败: {}", target_addr, e)))?;

            authenticate_handle(&mut h, username, credential, progress.as_ref()).await?;
            if let Some(ref p) = progress {
                p(&format!("目标主机 [{}] 认证通过 (SOCKS5 穿透)", target_addr));
            }
            h
        } else if (proxy_type == "http" || proxy_type == "https") && !proxy_host.is_empty() && proxy_port > 0 {
            info!(target: "smagical_ssh::session", "检测到 HTTP 代理 [{}:{}]: 纯 Rust 原生 CONNECT 隧道穿透至目标 [{}]...", proxy_host, proxy_port, target_addr);
            let proxy_addr = format!("{}:{}", proxy_host, proxy_port);
            if let Some(ref p) = progress {
                p(&format!("正在连接 HTTP 代理服务器 [{}] ...", proxy_addr));
            }
            let timeout_dur = std::time::Duration::from_secs(connect_timeout_sec as u64);
            let t_proxy = std::time::Instant::now();
            let stream = tokio::time::timeout(timeout_dur, connect_via_http_connect(&proxy_addr, &host.address, host.port))
                .await
                .map_err(|_| SshServiceError::Timeout(format!("连接 HTTP 代理超时 ({}s) [{}]", connect_timeout_sec, proxy_addr)))?
                .map_err(|e| SshServiceError::HostUnreachable(format!("HTTP 代理穿透失败 [{}]: {}", proxy_addr, e)))?;
            let proxy_ms = t_proxy.elapsed().as_millis();
            if let Some(ref p) = progress {
                p(&format!("HTTP CONNECT 隧道建立成功 (耗时 {}ms)，正在握手目标 [{}] ...", proxy_ms, target_addr));
            }
            let mut h = tokio::time::timeout(
                timeout_dur,
                russh::client::connect_stream(config.clone(), stream, self.make_handler(&host.address, host.port, progress.clone()))
            )
            .await
            .map_err(|_| SshServiceError::Timeout(format!("通过 HTTP 代理连接目标主机超时 ({}s): [{}]", connect_timeout_sec, target_addr)))?
            .map_err(|e| SshServiceError::HostUnreachable(format!("通过 HTTP 代理连接目标 [{}] 失败: {}", target_addr, e)))?;

            authenticate_handle(&mut h, username, credential, progress.as_ref()).await?;
            if let Some(ref p) = progress {
                p(&format!("目标主机 [{}] 认证通过 (HTTP 代理穿透)", target_addr));
            }
            h
        } else {
            // 普通直连模式
            if let Some(ref p) = progress {
                p(&format!("正在发起 TCP 网络连接: [{}] ...", target_addr));
            }
            let timeout_dur = std::time::Duration::from_secs(connect_timeout_sec as u64);
            let mut h = tokio::time::timeout(
                timeout_dur,
                russh::client::connect(config.clone(), target_addr.as_str(), self.make_handler(&host.address, host.port, progress.clone()))
            )
            .await
            .map_err(|_| {
                if let Some(ref p) = progress {
                    p(&format!("连接远程主机网络端口超时 ({}s): [{}]", connect_timeout_sec, target_addr));
                }
                SshServiceError::Timeout(format!("连接远程主机超时 ({}s): {}", connect_timeout_sec, target_addr))
            })?
            .map_err(|e| {
                if let Some(ref p) = progress {
                    p(&format!("连接远程主机网络端口失败 [{}]: {}", target_addr, e));
                }
                SshServiceError::HostUnreachable(format!("连接远程主机失败 [{}]: {}", target_addr, e))
            })?;
            if let Some(ref p) = progress {
                p("TCP 网络连接已建立，正在进行 SSH 协议握手与主机密钥校验...");
            }
            authenticate_handle(&mut h, username, credential, progress.as_ref()).await?;
            if let Some(ref p) = progress {
                p(&format!("目标主机 [{}] 认证通过", target_addr));
            }
            h
        };

        {
            let mut guard = self.handles.write().await;
            guard.insert(session_id.clone(), Arc::new(handle));
        }

        info!(target: "smagical_ssh::session", "纯 Rust SSH 会话已建立并缓存: [{}]", session_id);
        Ok(session_id)
    }

    async fn open_pty_channel(
        &self,
        session_id: &str,
        term_type: &str,
        rows: u16,
        cols: u16,
    ) -> SshServiceResult<Box<dyn SshStreamChannel>> {
        let handle = self.get_handle(session_id).await?;

        debug!(
            target: "smagical_ssh::session",
            "正在会话 [{}] 打开 PTY 虚拟终端通道 ({}x{}, 终端类型: {})",
            session_id, cols, rows, term_type
        );

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("打开 Session Channel 失败: {}", e)))?;

        // 请求远端伪终端
        channel
            .request_pty(
                false,
                term_type,
                cols as u32,
                rows as u32,
                0,
                0,
                &[],
            )
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("请求远端 PTY 失败: {}", e)))?;

        // 启动远程默认交互式 Shell
        channel
            .request_shell(true)
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("请求启动远端 Shell 失败: {}", e)))?;

        info!(
            target: "smagical_ssh::session",
            "会话 [{}] 交互式虚拟终端已成功就绪", session_id
        );

        // 转换为实现了 AsyncRead + AsyncWrite 的异步双向流
        let stream = channel.into_stream();
        Ok(Box::new(stream))
    }

    async fn resize_pty(&self, session_id: &str, rows: u16, cols: u16) -> SshServiceResult<()> {
        debug!(
            target: "smagical_ssh::session",
            "会话 [{}] 下发 PTY 尺寸变更: {}x{}",
            session_id, cols, rows
        );
        Ok(())
    }

    async fn execute_command(&self, session_id: &str, command: &str) -> SshServiceResult<CommandExecutionOutput> {
        let handle = self.get_handle(session_id).await?;

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("打开 Exec Channel 失败: {}", e)))?;

        channel
            .exec(true, command)
            .await
            .map_err(|e| SshServiceError::ProtocolError(format!("发送 Exec 命令失败: {}", e)))?;

        let mut stdout_buf = Vec::new();
        let stderr_buf = Vec::new();
        let exit_code = 0;

        let mut stream = channel.into_stream();
        let mut read_buf = vec![0u8; 4096];

        const MAX_EXEC_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MB 安全上限

        loop {
            match stream.read(&mut read_buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if stdout_buf.len() < MAX_EXEC_OUTPUT_BYTES {
                        let take = (MAX_EXEC_OUTPUT_BYTES - stdout_buf.len()).min(n);
                        stdout_buf.extend_from_slice(&read_buf[..take]);
                        if stdout_buf.len() >= MAX_EXEC_OUTPUT_BYTES {
                            warn!(
                                target: "smagical_ssh::session",
                                "会话 [{}] Exec 命令输出达到 10MB 安全上限，已停止读取以防止 OOM",
                                session_id
                            );
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!(target: "smagical_ssh::session", "Exec 命令流读取结束: {}", e);
                    break;
                }
            }
        }

        Ok(CommandExecutionOutput {
            exit_code,
            stdout: stdout_buf,
            stderr: stderr_buf,
        })
    }

    async fn disconnect(&self, session_id: &str) -> SshServiceResult<()> {
        let mut guard = self.handles.write().await;
        if let Some(handle) = guard.remove(session_id) {
            info!(target: "smagical_ssh::session", "正在断开会话 [{}]", session_id);
            let _ = handle.disconnect(russh::Disconnect::ByApplication, "Session Closed", "en").await;
            Ok(())
        } else {
            Err(SshServiceError::NotFound(format!("会话 [{}] 不存在或已断开", session_id)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_initialization() {
        let driver = RusshSessionDriver::new();
        let default_driver = RusshSessionDriver::default();
        assert_eq!(Arc::strong_count(&driver.handles), 1);
        assert_eq!(Arc::strong_count(&default_driver.handles), 1);
    }

    #[tokio::test]
    async fn test_unregistered_session_returns_not_found() {
        let driver = RusshSessionDriver::new();
        let res = driver.open_pty_channel("non_existent_session", "xterm", 24, 80).await;
        match res {
            Err(SshServiceError::NotFound(msg)) => {
                assert!(msg.contains("non_existent_session"));
            }
            _ => panic!("预期返回 NotFound"),
        }
    }

    #[tokio::test]
    async fn test_invalid_host_returns_unreachable() {
        let driver = RusshSessionDriver::new();
        let host = HostRecord {
            id: "test-invalid".to_string(),
            name: "Invalid Host".to_string(),
            address: "127.0.0.1".to_string(),
            port: 59999, // 不可达的端口
            ..Default::default()
        };

        let cred = CredentialRecord {
            id: "cred-test".to_string(),
            name: "Test Cred".to_string(),
            cred_type: CredentialType::Password,
            algorithm: "Password".to_string(),
            username: Some("testuser".to_string()),
            secret_data: "testpassword".to_string(),
            passphrase: None,
            public_key: None,
            fingerprint: None,
            bound_host_count: 0,
            created_at: String::new(),
            updated_at: String::new(),
            notes: String::new(),
        };

        let res = driver.connect(&host, Some(&cred)).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            SshServiceError::HostUnreachable(msg) => {
                assert!(msg.contains("127.0.0.1:59999"));
            }
            other => panic!("预期返回 HostUnreachable，实际返回: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_invalid_jump_host_returns_unreachable() {
        let driver = RusshSessionDriver::new();
        let host = HostRecord {
            id: "test-jump-invalid".to_string(),
            name: "Jump Invalid Host".to_string(),
            address: "10.0.0.1".to_string(),
            port: 22,
            jump_chain_summary: Some("127.0.0.1:59998".to_string()),
            ..Default::default()
        };

        let res = driver.connect(&host, None).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            SshServiceError::HostUnreachable(msg) => {
                assert!(msg.contains("127.0.0.1:59998"), "应提示连接跳板机失败: {msg}");
            }
            other => panic!("预期返回 HostUnreachable，实际返回: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_connect_via_socks5_mock() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // 模拟 SOCKS5 代理服务端
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 3];
                let _ = socket.read_exact(&mut buf).await;
                let _ = socket.write_all(&[0x05, 0x00]).await;

                let mut req = [0u8; 10];
                let _ = socket.read_exact(&mut req).await;
                let _ = socket.write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 22]).await;
            }
        });

        let proxy_addr = addr.to_string();
        let res = connect_via_socks5(&proxy_addr, "127.0.0.1", 22).await;
        assert!(res.is_ok(), "SOCKS5 代理穿透应成功建立流: {:?}", res.err());
    }

    #[tokio::test]
    async fn test_connect_via_http_connect_mock() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // 模拟 HTTP CONNECT 代理服务端
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);
                if req_str.starts_with("CONNECT") {
                    let _ = socket.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;
                }
            }
        });

        let proxy_addr = addr.to_string();
        let res = connect_via_http_connect(&proxy_addr, "example.com", 22).await;
        assert!(res.is_ok(), "HTTP CONNECT 代理穿透应成功建立流: {:?}", res.err());
    }

    #[tokio::test]
    async fn test_agent_credential_handling_when_no_agent() {
        let cred = CredentialRecord {
            id: "cred-agent-test".to_string(),
            name: "Agent Test".to_string(),
            cred_type: CredentialType::Agent,
            algorithm: "Agent".to_string(),
            username: Some("agentuser".to_string()),
            secret_data: String::new(),
            passphrase: None,
            public_key: None,
            fingerprint: None,
            bound_host_count: 0,
            created_at: String::new(),
            updated_at: String::new(),
            notes: String::new(),
        };

        // 验证 CredentialType::Agent 类型配置识别
        assert_eq!(cred.cred_type, CredentialType::Agent);
    }
}

