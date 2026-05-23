//! SSH executor 运行状态。

use std::collections::HashMap;

use tokio::runtime::Runtime;

use crate::model::SessionId;
use crate::security::SecretStore;

use super::super::{RemoteSftp, RemoteShell, RemoteTunnel, RusshConnection, RusshConnector};

/// 将同步后端执行器接口适配到异步 `russh` 客户端。
pub struct RusshBackendExecutor<S: SecretStore + Send> {
    pub(super) runtime: Runtime,
    pub(super) connector: RusshConnector,
    pub(super) secret_store: S,
    pub(super) connections: HashMap<SessionId, RusshConnection>,
    pub(super) shells: HashMap<SessionId, RemoteShell>,
    pub(super) sftps: HashMap<SessionId, RemoteSftp>,
    pub(super) tunnels: HashMap<String, RemoteTunnel>,
}

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    /// 创建使用默认连接器的真实 SSH 执行器。
    pub fn new(secret_store: S) -> std::io::Result<Self> {
        Self::with_connector(secret_store, RusshConnector::new())
    }

    /// 创建使用指定连接器的真实 SSH 执行器。
    pub fn with_connector(secret_store: S, connector: RusshConnector) -> std::io::Result<Self> {
        Ok(Self {
            runtime: Runtime::new()?,
            connector,
            secret_store,
            connections: HashMap::new(),
            shells: HashMap::new(),
            sftps: HashMap::new(),
            tunnels: HashMap::new(),
        })
    }

    /// 当前缓存的已认证 SSH 连接数量。
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// 当前缓存的交互式 shell 数量。
    pub fn shell_count(&self) -> usize {
        self.shells.len()
    }

    /// 当前缓存的 SFTP 子系统会话数量。
    pub fn sftp_count(&self) -> usize {
        self.sftps.len()
    }

    /// 当前运行中的隧道数量。
    pub fn tunnel_count(&self) -> usize {
        self.tunnels.len()
    }
}
