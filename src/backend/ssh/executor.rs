//! `russh` 后端执行器。

use std::collections::HashMap;

use tokio::runtime::Runtime;

use crate::backend::{
    BackendCommand, BackendEvent, BackendExecutionError, BackendExecutor, ConnectionTarget,
    PtyRequest, RemoteCommandRequest, SftpRequest, TunnelStartRequest, TunnelStopRequest,
};
use crate::model::SessionId;
use crate::security::SecretStore;

use super::{
    RemoteSftp, RemoteShell, RemoteTunnel, RusshConnection, RusshConnector, SshConnectionPlan,
};

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

    fn send_shell_input(
        &mut self,
        session_id: SessionId,
        input: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let runtime = &self.runtime;
        let shell = self
            .shells
            .get(&session_id)
            .ok_or_else(|| connected_session_error("send shell input"))?;
        runtime.block_on(shell.send_input(input.as_bytes()))?;
        Ok(Vec::new())
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

    fn sftp(
        &mut self,
        session_id: SessionId,
        request: SftpRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        if !self.sftps.contains_key(&session_id) {
            let runtime = &self.runtime;
            let connection = self
                .connections
                .get_mut(&session_id)
                .ok_or_else(|| connected_session_error("sftp"))?;
            let sftp = runtime.block_on(connection.open_sftp(session_id))?;
            self.sftps.insert(session_id, sftp);
        }

        let sftp = self
            .sftps
            .get(&session_id)
            .ok_or_else(|| connected_session_error("sftp"))?;
        self.runtime.block_on(sftp.execute(request))
    }

    fn start_tunnel(
        &mut self,
        session_id: SessionId,
        request: TunnelStartRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let connection = self
            .connections
            .remove(&session_id)
            .ok_or_else(|| connected_session_error("start tunnel"))?;
        let (tunnel, events) = self
            .runtime
            .block_on(connection.into_tunnel(session_id, request))?;
        self.tunnels.insert(tunnel.rule_name().to_owned(), tunnel);
        Ok(events)
    }

    fn stop_tunnel(
        &mut self,
        session_id: SessionId,
        request: TunnelStopRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let rule_name = request.rule_name;
        if let Some(tunnel) = self.tunnels.remove(&rule_name) {
            tunnel.stop();
        }

        Ok(vec![BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name,
            status: crate::model::TunnelStatus::Stopped,
        }])
    }

    fn disconnect(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        self.shells.remove(&session_id);

        if let Some(sftp) = self.sftps.remove(&session_id) {
            self.runtime.block_on(sftp.close())?;
        }

        if let Some(connection) = self.connections.remove(&session_id) {
            self.runtime.block_on(connection.disconnect())?;
        }

        Ok(vec![BackendEvent::Disconnected { session_id }])
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

fn connected_session_error(operation: &str) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: "session is not connected".to_owned(),
    }
}
