//! 基于 `russh` 的真实 SSH 客户端边界。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client;
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::{Certificate, HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey};

use crate::model::{HostKeyVerification, KnownHostEntry, SessionId};

use super::super::{BackendEvent, BackendExecutionError};
use super::{SshAuthPlan, SshConnectionPlan};

mod session;
pub use session::*;

#[cfg(test)]
mod tests;

const DEFAULT_INACTIVITY_TIMEOUT_SECS: u64 = 30;
const DEFAULT_KEEPALIVE_INTERVAL_SECS: u64 = 15;
const DEFAULT_KEEPALIVE_MAX: usize = 3;
#[cfg(windows)]
const WINDOWS_OPENSSH_AGENT_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";
type DynamicAgentClient = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

/// `russh` 客户端配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshClientSettings {
    pub inactivity_timeout: Duration,
    pub keepalive_interval: Option<Duration>,
    pub keepalive_max: usize,
    pub nodelay: bool,
}

impl Default for RusshClientSettings {
    fn default() -> Self {
        Self {
            inactivity_timeout: Duration::from_secs(DEFAULT_INACTIVITY_TIMEOUT_SECS),
            keepalive_interval: Some(Duration::from_secs(DEFAULT_KEEPALIVE_INTERVAL_SECS)),
            keepalive_max: DEFAULT_KEEPALIVE_MAX,
            nodelay: true,
        }
    }
}

impl RusshClientSettings {
    /// 转换为 `russh` 原生客户端配置。
    pub fn to_russh_config(&self) -> client::Config {
        client::Config {
            inactivity_timeout: Some(self.inactivity_timeout),
            keepalive_interval: self.keepalive_interval,
            keepalive_max: self.keepalive_max,
            nodelay: self.nodelay,
            ..Default::default()
        }
    }
}

/// 主机密钥校验策略。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyPolicy {
    /// 明确允许未知主机密钥，后续应接入 Known Hosts 首次信任确认。
    AcceptAny,
    /// 只允许已信任的 Known Hosts 记录。
    KnownHosts(Vec<KnownHostEntry>),
}

impl Default for HostKeyPolicy {
    fn default() -> Self {
        Self::AcceptAny
    }
}

impl HostKeyPolicy {
    /// 校验服务端主机密钥并返回是否允许连接。
    pub fn check(&self, host: &str, port: u16, public_key: &PublicKey) -> HostKeyCheck {
        let fingerprint = host_key_fingerprint(public_key);

        match self {
            Self::AcceptAny => HostKeyCheck {
                verification: HostKeyVerification::Unknown,
                accepted: true,
                fingerprint,
            },
            Self::KnownHosts(entries) => {
                let verification = entries
                    .iter()
                    .find(|entry| entry.host == host && entry.port == port)
                    .map(|entry| entry.verify(host, port, &fingerprint))
                    .unwrap_or(HostKeyVerification::Unknown);

                HostKeyCheck {
                    accepted: matches!(verification, HostKeyVerification::Trusted),
                    verification,
                    fingerprint,
                }
            }
        }
    }
}

/// 单次主机密钥校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyCheck {
    pub verification: HostKeyVerification,
    pub accepted: bool,
    pub fingerprint: String,
}

/// `russh` 连接器。
#[derive(Debug, Clone, Default)]
pub struct RusshConnector {
    settings: RusshClientSettings,
    host_key_policy: HostKeyPolicy,
}

impl RusshConnector {
    /// 使用默认配置创建连接器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用指定配置创建连接器。
    pub fn with_settings(settings: RusshClientSettings) -> Self {
        Self {
            settings,
            host_key_policy: HostKeyPolicy::default(),
        }
    }

    /// 设置主机密钥校验策略。
    pub fn with_host_key_policy(mut self, host_key_policy: HostKeyPolicy) -> Self {
        self.host_key_policy = host_key_policy;
        self
    }

    /// 建立连接并完成认证，返回后续命令可复用的会话句柄。
    pub async fn connect(
        &self,
        session_id: SessionId,
        plan: SshConnectionPlan,
    ) -> Result<RusshConnectionReport, BackendExecutionError> {
        let mut events = vec![BackendEvent::Connecting {
            session_id,
            endpoint: plan.endpoint.clone(),
        }];
        let host_key_result = SharedHostKeyResult::default();
        let handler = SshClientHandler::new(
            plan.host.clone(),
            plan.port,
            self.host_key_policy.clone(),
            host_key_result.clone(),
        );
        let config = Arc::new(self.settings.to_russh_config());
        let address = (plan.host.as_str(), plan.port);

        let mut handle = client::connect(config, address, handler)
            .await
            .map_err(|error| connection_error(&plan.endpoint, error))?;

        if let Some(result) = host_key_result.get() {
            events.push(BackendEvent::HostKeyVerified { session_id, result });
        }

        events.push(BackendEvent::Authenticating {
            session_id,
            username: plan.username().to_owned(),
        });
        authenticate(&mut handle, &plan.auth).await?;
        events.push(BackendEvent::Authenticated { session_id });
        events.push(BackendEvent::Connected { session_id });

        Ok(RusshConnectionReport {
            connection: RusshConnection {
                handle,
                endpoint: plan.endpoint,
                username: plan.auth.username().to_owned(),
            },
            events,
        })
    }
}

/// 成功连接后的结果。
pub struct RusshConnectionReport {
    pub connection: RusshConnection,
    pub events: Vec<BackendEvent>,
}

/// 已认证 SSH 连接。
pub struct RusshConnection {
    handle: client::Handle<SshClientHandler>,
    endpoint: String,
    username: String,
}

impl RusshConnection {
    /// 返回连接端点。
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// 返回认证用户名。
    pub fn username(&self) -> &str {
        &self.username
    }

    /// 返回底层 `russh` 句柄，供后续 shell、命令、SFTP 和隧道模块复用。
    pub fn handle_mut(&mut self) -> &mut client::Handle<SshClientHandler> {
        &mut self.handle
    }

    /// 主动断开连接。
    pub async fn disconnect(&self) -> Result<(), BackendExecutionError> {
        self.handle
            .disconnect(russh::Disconnect::ByApplication, "", "zh-CN")
            .await
            .map_err(|error| BackendExecutionError::ConnectionFailed {
                endpoint: self.endpoint.clone(),
                reason: error.to_string(),
            })
    }
}

#[derive(Debug, Clone)]
pub struct SshClientHandler {
    host: String,
    port: u16,
    host_key_policy: HostKeyPolicy,
    host_key_result: SharedHostKeyResult,
}

impl SshClientHandler {
    fn new(
        host: String,
        port: u16,
        host_key_policy: HostKeyPolicy,
        host_key_result: SharedHostKeyResult,
    ) -> Self {
        Self {
            host,
            port,
            host_key_policy,
            host_key_result,
        }
    }
}

impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let check = self
            .host_key_policy
            .check(&self.host, self.port, server_public_key);
        self.host_key_result.set(check.verification);
        Ok(check.accepted)
    }
}

#[derive(Debug, Clone, Default)]
struct SharedHostKeyResult {
    value: Arc<Mutex<Option<HostKeyVerification>>>,
}

impl SharedHostKeyResult {
    fn set(&self, result: HostKeyVerification) {
        if let Ok(mut value) = self.value.lock() {
            *value = Some(result);
        }
    }

    fn get(&self) -> Option<HostKeyVerification> {
        self.value.lock().ok().and_then(|value| value.clone())
    }
}

async fn authenticate(
    handle: &mut client::Handle<SshClientHandler>,
    auth: &SshAuthPlan,
) -> Result<(), BackendExecutionError> {
    let result = match auth {
        SshAuthPlan::Password { username, password } => handle
            .authenticate_password(username.clone(), password.clone())
            .await
            .map_err(|error| authentication_error(username, error))?,
        SshAuthPlan::Key {
            username,
            private_key,
            passphrase,
        } => {
            let key = decode_private_key(private_key, passphrase.as_deref(), username)?;
            let hash_alg = best_supported_rsa_hash(handle, username).await?;
            handle
                .authenticate_publickey(
                    username.clone(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg),
                )
                .await
                .map_err(|error| authentication_error(username, error))?
        }
        SshAuthPlan::Agent { username, key_hint } => {
            authenticate_agent(handle, username, key_hint.as_deref()).await?
        }
        SshAuthPlan::Certificate {
            username,
            private_key,
            certificate,
        } => {
            let key = decode_private_key(private_key, None, username)?;
            let certificate = Certificate::from_openssh(certificate)
                .map_err(|error| authentication_error(username, error))?;
            handle
                .authenticate_openssh_cert(username.clone(), Arc::new(key), certificate)
                .await
                .map_err(|error| authentication_error(username, error))?
        }
    };

    if result.success() {
        Ok(())
    } else {
        Err(BackendExecutionError::AuthenticationFailed {
            username: auth.username().to_owned(),
            reason: format!("{} 认证被服务器拒绝", auth.method()),
        })
    }
}

async fn authenticate_agent(
    handle: &mut client::Handle<SshClientHandler>,
    username: &str,
    key_hint: Option<&str>,
) -> Result<client::AuthResult, BackendExecutionError> {
    let mut agent = connect_agent()
        .await
        .map_err(|error| authentication_error(username, error))?;
    let identities = agent
        .request_identities()
        .await
        .map_err(|error| authentication_error(username, error))?;
    let public_key = select_agent_identity(&identities, key_hint).ok_or_else(|| {
        BackendExecutionError::AuthenticationFailed {
            username: username.to_owned(),
            reason: agent_identity_error(key_hint),
        }
    })?;
    let hash_alg = best_supported_rsa_hash(handle, username).await?;

    handle
        .authenticate_publickey_with(username.to_owned(), public_key, hash_alg, &mut agent)
        .await
        .map_err(|error| authentication_error(username, error))
}

async fn best_supported_rsa_hash(
    handle: &client::Handle<SshClientHandler>,
    username: &str,
) -> Result<Option<HashAlg>, BackendExecutionError> {
    handle
        .best_supported_rsa_hash()
        .await
        .map(|hash_alg| hash_alg.flatten())
        .map_err(|error| authentication_error(username, error))
}

fn decode_private_key(
    private_key: &str,
    passphrase: Option<&str>,
    username: &str,
) -> Result<PrivateKey, BackendExecutionError> {
    russh::keys::decode_secret_key(private_key, passphrase)
        .map_err(|error| authentication_error(username, error))
}

async fn connect_agent() -> Result<DynamicAgentClient, russh::keys::Error> {
    #[cfg(unix)]
    {
        AgentClient::connect_env().await.map(AgentClient::dynamic)
    }

    #[cfg(windows)]
    {
        match AgentClient::connect_named_pipe(WINDOWS_OPENSSH_AGENT_PIPE).await {
            Ok(agent) => Ok(agent.dynamic()),
            Err(named_pipe_error) => match AgentClient::connect_pageant().await {
                Ok(agent) => Ok(agent.dynamic()),
                Err(pageant_error) => Err(russh::keys::Error::IO(std::io::Error::other(format!(
                    "OpenSSH agent 连接失败：{named_pipe_error}; Pageant 连接失败：{pageant_error}"
                )))),
            },
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(russh::keys::Error::IO(std::io::Error::other(
            "当前平台暂不支持 ssh-agent 自动发现",
        )))
    }
}

fn select_agent_identity(identities: &[PublicKey], key_hint: Option<&str>) -> Option<PublicKey> {
    match key_hint {
        Some(hint) => identities
            .iter()
            .find(|identity| agent_identity_matches(identity, hint))
            .cloned(),
        None => identities.first().cloned(),
    }
}

fn agent_identity_matches(identity: &PublicKey, hint: &str) -> bool {
    let fingerprint = host_key_fingerprint(identity);
    identity.comment().contains(hint)
        || fingerprint == hint
        || format!("{:?}", identity.algorithm()).contains(hint)
}

fn agent_identity_error(key_hint: Option<&str>) -> String {
    match key_hint {
        Some(hint) => format!("ssh-agent 中没有匹配的身份：{hint}"),
        None => "ssh-agent 中没有可用身份".to_owned(),
    }
}

fn host_key_fingerprint(public_key: &PublicKey) -> String {
    public_key.fingerprint(HashAlg::Sha256).to_string()
}

fn connection_error(endpoint: &str, error: russh::Error) -> BackendExecutionError {
    BackendExecutionError::ConnectionFailed {
        endpoint: endpoint.to_owned(),
        reason: error.to_string(),
    }
}

fn authentication_error(username: &str, error: impl std::error::Error) -> BackendExecutionError {
    BackendExecutionError::AuthenticationFailed {
        username: username.to_owned(),
        reason: error.to_string(),
    }
}
