//! `russh` 后端执行器。

use std::collections::HashMap;

use tokio::runtime::Runtime;

use crate::backend::{
    BackendCommand, BackendEvent, BackendExecutionError, BackendExecutor, ConnectionTarget,
    PtyRequest, RemoteCommandRequest,
};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::{RemoteShell, RusshConnection, RusshConnector, SshConnectionPlan};

#[cfg(test)]
mod tests;

/// 将同步后端执行器接口适配到异步 `russh` 客户端。
pub struct RusshBackendExecutor<S: SecretStore> {
    runtime: Runtime,
    connector: RusshConnector,
    secret_store: S,
    connections: HashMap<SessionId, RusshConnection>,
    shells: HashMap<SessionId, RemoteShell>,
}

impl<S: SecretStore> RusshBackendExecutor<S> {
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

    fn connect(
        &mut self,
        session_id: SessionId,
        target: ConnectionTarget,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let plan = SshConnectionPlan::from_target(&target, &self.secret_store)?;
        let report = self
            .runtime
            .block_on(self.connector.connect(session_id, plan))?;
        self.connections.insert(session_id, report.connection);
        Ok(report.events)
    }

    fn open_shell(
        &mut self,
        session_id: SessionId,
        pty: PtyRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let runtime = &self.runtime;
        let connection = self
            .connections
            .get_mut(&session_id)
            .ok_or_else(|| connected_session_error("open shell"))?;
        let report = runtime.block_on(connection.open_shell(session_id, &pty))?;
        self.shells.insert(session_id, report.shell);
        Ok(report.events)
    }

    fn run_command(
        &mut self,
        session_id: SessionId,
        request: RemoteCommandRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let runtime = &self.runtime;
        let connection = self
            .connections
            .get_mut(&session_id)
            .ok_or_else(|| connected_session_error("run command"))?;
        runtime.block_on(connection.run_command(session_id, &request))
    }

    fn disconnect(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        self.shells.remove(&session_id);

        if let Some(connection) = self.connections.remove(&session_id) {
            self.runtime.block_on(connection.disconnect())?;
        }

        Ok(vec![BackendEvent::Disconnected { session_id }])
    }
}

impl<S: SecretStore> BackendExecutor for RusshBackendExecutor<S> {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let kind = command.kind();

        match command {
            BackendCommand::Connect { session_id, target } => self.connect(session_id, target),
            BackendCommand::OpenShell { session_id, pty } => self.open_shell(session_id, pty),
            BackendCommand::RunCommand {
                session_id,
                request,
            } => self.run_command(session_id, request),
            BackendCommand::Disconnect { session_id } => self.disconnect(session_id),
            BackendCommand::Sftp { .. }
            | BackendCommand::StartTunnel { .. }
            | BackendCommand::StopTunnel { .. } => {
                Err(BackendExecutionError::UnsupportedCommand { kind })
            }
        }
    }
}

fn connected_session_error(operation: &str) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: "session is not connected".to_owned(),
    }
}
