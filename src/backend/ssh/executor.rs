//! `russh` 后端执行器。

use std::collections::HashMap;
use tokio::runtime::Runtime;

use crate::backend::{BackendCommand, BackendEvent, BackendExecutionError, BackendExecutor};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::{RemoteSftp, RemoteShell, RemoteTunnel, RusshConnection, RusshConnector};

mod cache;
mod session_runtime;
mod sftp_runtime;
mod shell_runtime;
mod tunnel_runtime;

#[cfg(test)]
mod tests;

/// 将同步后端执行器接口适配到异步 `russh` 客户端。
pub struct RusshBackendExecutor<S: SecretStore + Send> {
    runtime: Runtime,
    connector: RusshConnector,
    secret_store: S,
    connections: HashMap<SessionId, RusshConnection>,
    shells: HashMap<SessionId, RemoteShell>,
    sftps: HashMap<SessionId, RemoteSftp>,
    tunnels: HashMap<String, RemoteTunnel>,
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

impl<S: SecretStore + Send> BackendExecutor for RusshBackendExecutor<S> {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        match command {
            BackendCommand::Connect { session_id, target } => self.connect(session_id, target),
            BackendCommand::OpenShell { session_id, pty } => self.open_shell(session_id, pty),
            BackendCommand::RunCommand {
                session_id,
                request,
            } => self.run_command(session_id, request),
            BackendCommand::SendShellInput { session_id, input } => {
                self.send_shell_input(session_id, input)
            }
            BackendCommand::DrainSessionOutput { session_id } => {
                self.drain_session_output(session_id)
            }
            BackendCommand::Sftp {
                session_id,
                request,
            } => self.sftp(session_id, request),
            BackendCommand::StartTunnel {
                session_id,
                request,
            } => self.start_tunnel(session_id, request),
            BackendCommand::StopTunnel {
                session_id,
                request,
            } => self.stop_tunnel(session_id, request),
            BackendCommand::Disconnect { session_id } => self.disconnect(session_id),
        }
    }
}
