//! 已认证 SSH 连接。

use russh::client;
use smagical_ssh_client_core::connection_error;
use tokio::sync::mpsc;

use super::SshClientHandler;
use super::handler::{ForwardedChannel, SharedForwardedChannels};
use crate::backend::{BackendEvent, BackendExecutionError};

/// 成功连接后的结果。
pub struct RusshConnectionReport {
    pub connection: RusshConnection,
    pub events: Vec<BackendEvent>,
}

/// 已认证 SSH 连接。
pub struct RusshConnection {
    pub(super) handle: client::Handle<SshClientHandler>,
    pub(super) endpoint: String,
    pub(super) username: String,
    pub(super) forwarded_channels: SharedForwardedChannels,
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
            .map_err(|error| connection_error(&self.endpoint, error))
    }
}
