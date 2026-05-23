//! `russh` 连接器。

use std::sync::Arc;

use russh::client;
use smagical_ssh_client_core::{
    authenticated_event, authenticating_event, connected_event, connecting_event,
    host_key_or_connection_error, host_key_policy_for_known_hosts, host_key_verified_event,
};

use super::auth::authenticate;
use super::connection::{RusshConnection, RusshConnectionReport};
use super::handler::{SharedForwardedChannels, SharedHostKeyResult};
use super::{HostKeyPolicy, RusshClientSettings, SshClientHandler};
use crate::backend::{BackendExecutionError, SshConnectionPlan};
use crate::model::SessionId;

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
        let mut events = vec![connecting_event(session_id, plan.endpoint.clone())];
        let host_key_result = SharedHostKeyResult::default();
        let forwarded_channels = SharedForwardedChannels::default();
        let handler = SshClientHandler::new(
            plan.host.clone(),
            plan.port,
            host_key_policy_for_known_hosts(&self.host_key_policy, &plan.known_hosts),
            host_key_result.clone(),
            forwarded_channels.clone(),
        );
        let config = Arc::new(self.settings.to_russh_config());
        let address = (plan.host.as_str(), plan.port);

        let mut handle = client::connect(config, address, handler)
            .await
            .map_err(|error| {
                host_key_or_connection_error(&plan.endpoint, &host_key_result, error)
            })?;

        if let Some(check) = host_key_result.get() {
            events.push(host_key_verified_event(session_id, check));
        }

        events.push(authenticating_event(session_id, plan.username().to_owned()));
        authenticate(&mut handle, &plan.auth).await?;
        events.push(authenticated_event(session_id));
        events.push(connected_event(session_id));

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
