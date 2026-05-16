//! 基于 `russh` 的真实 SSH 客户端边界。

use std::sync::Arc;

use russh::client;
use tokio::sync::mpsc;

use crate::model::SessionId;

use super::super::{BackendEvent, BackendExecutionError};
use super::SshConnectionPlan;

mod auth;
mod handler;
mod host_key;
mod session;
mod settings;

use auth::authenticate;
use handler::{ForwardedChannel, SharedForwardedChannels, SharedHostKeyResult};

pub use handler::SshClientHandler;
pub use host_key::*;
pub use session::*;
pub use settings::RusshClientSettings;

#[cfg(test)]
mod tests;

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
        let forwarded_channels = SharedForwardedChannels::default();
        let handler = SshClientHandler::new(
            plan.host.clone(),
            plan.port,
            self.host_key_policy.clone(),
            host_key_result.clone(),
            forwarded_channels.clone(),
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
                forwarded_channels,
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
    forwarded_channels: SharedForwardedChannels,
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

    /// 订阅指定远程转发规则收到的服务端 forwarded-tcpip channel。
    pub fn subscribe_forwarded_channels(
        &self,
        bind_host: &str,
        bind_port: u16,
    ) -> mpsc::UnboundedReceiver<ForwardedChannel> {
        self.forwarded_channels.subscribe(bind_host, bind_port)
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

fn connection_error(endpoint: &str, error: russh::Error) -> BackendExecutionError {
    BackendExecutionError::ConnectionFailed {
        endpoint: endpoint.to_owned(),
        reason: error.to_string(),
    }
}
