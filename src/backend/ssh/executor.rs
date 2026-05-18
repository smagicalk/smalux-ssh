//! `russh` 后端执行器。

use std::collections::HashMap;
use std::time::Duration;

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

const REMOTE_SHELL_DRAIN_MAX_EVENTS: usize = 64;
const REMOTE_SHELL_DRAIN_POLL_TIMEOUT: Duration = Duration::from_millis(1);

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
        let stale_resources = take_cached_session_resources(
            &mut self.shells,
            &mut self.sftps,
            &mut self.connections,
            session_id,
        );
        self.close_stale_session_resources(session_id, stale_resources);
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
        let result = runtime.block_on(shell.send_input(input.as_bytes()));
        drop_cached_shell_after_failed_input(&mut self.shells, session_id, &result);
        result?;
        Ok(Vec::new())
    }

    fn drain_session_output(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let runtime = &self.runtime;
        let Some(shell) = self.shells.get_mut(&session_id) else {
            return Ok(Vec::new());
        };
        let events = runtime.block_on(shell.drain_ready_events(
            REMOTE_SHELL_DRAIN_MAX_EVENTS,
            REMOTE_SHELL_DRAIN_POLL_TIMEOUT,
        ));

        if events.iter().any(BackendEvent::is_terminal) {
            self.shells.remove(&session_id);
        }

        Ok(events)
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
        let result = self.runtime.block_on(sftp.execute(request));
        drop_cached_sftp_after_failed_request(&mut self.sftps, session_id, &result);
        result
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
        let stale_subresources =
            take_cached_session_subresources(&mut self.shells, &mut self.sftps, session_id);
        self.close_stale_session_subresources(session_id, stale_subresources, "starting tunnel");
        let (tunnel, events) = self
            .runtime
            .block_on(connection.into_tunnel(session_id, request))?;
        replace_tunnel_stopping_previous(&mut self.tunnels, tunnel);
        Ok(events)
    }

    fn stop_tunnel(
        &mut self,
        session_id: SessionId,
        request: TunnelStopRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let rule_name = request.rule_name;
        if let Some(tunnel) =
            remove_tunnel_for_session_rule(&mut self.tunnels, session_id, &rule_name)
        {
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
        let resources = take_cached_session_resources(
            &mut self.shells,
            &mut self.sftps,
            &mut self.connections,
            session_id,
        );
        self.close_disconnected_session_resources(session_id, resources);

        Ok(vec![BackendEvent::Disconnected { session_id }])
    }

    fn close_stale_session_resources(
        &self,
        session_id: SessionId,
        resources: CachedSessionResources<RemoteShell, RemoteSftp, RusshConnection>,
    ) {
        self.close_stale_session_subresources(
            session_id,
            CachedSessionSubresources {
                shell: resources.shell,
                sftp: resources.sftp,
            },
            "reconnecting",
        );

        if let Some(connection) = resources.connection
            && let Err(error) = self.runtime.block_on(connection.disconnect())
        {
            tracing::warn!(
                session_id = %session_id.0,
                error = %error,
                "failed to disconnect stale SSH connection before reconnect"
            );
        }
    }

    fn close_disconnected_session_resources(
        &self,
        session_id: SessionId,
        resources: CachedSessionResources<RemoteShell, RemoteSftp, RusshConnection>,
    ) {
        self.close_stale_session_subresources(
            session_id,
            CachedSessionSubresources {
                shell: resources.shell,
                sftp: resources.sftp,
            },
            "disconnecting",
        );

        if let Some(connection) = resources.connection
            && let Err(error) = self.runtime.block_on(connection.disconnect())
        {
            tracing::warn!(
                session_id = %session_id.0,
                error = %error,
                "failed to disconnect SSH connection"
            );
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CachedSessionSubresources<TShell, TSftp> {
    shell: Option<TShell>,
    sftp: Option<TSftp>,
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

fn connected_session_error(operation: &str) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: "session is not connected".to_owned(),
    }
}

impl<S: SecretStore + Send> RusshBackendExecutor<S> {
    fn close_stale_session_subresources(
        &self,
        session_id: SessionId,
        resources: CachedSessionSubresources<RemoteShell, RemoteSftp>,
        operation: &'static str,
    ) {
        if resources.shell.is_some() {
            tracing::warn!(
                session_id = %session_id.0,
                operation,
                "dropping stale remote shell"
            );
        }

        if let Some(sftp) = resources.sftp
            && let Err(error) = self.runtime.block_on(sftp.close())
        {
            tracing::warn!(
                session_id = %session_id.0,
                operation,
                error = %error,
                "failed to close stale SFTP session"
            );
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CachedSessionResources<TShell, TSftp, TConnection> {
    shell: Option<TShell>,
    sftp: Option<TSftp>,
    connection: Option<TConnection>,
}

fn take_cached_session_subresources<TShell, TSftp>(
    shells: &mut HashMap<SessionId, TShell>,
    sftps: &mut HashMap<SessionId, TSftp>,
    session_id: SessionId,
) -> CachedSessionSubresources<TShell, TSftp> {
    CachedSessionSubresources {
        shell: shells.remove(&session_id),
        sftp: sftps.remove(&session_id),
    }
}

fn take_cached_session_resources<TShell, TSftp, TConnection>(
    shells: &mut HashMap<SessionId, TShell>,
    sftps: &mut HashMap<SessionId, TSftp>,
    connections: &mut HashMap<SessionId, TConnection>,
    session_id: SessionId,
) -> CachedSessionResources<TShell, TSftp, TConnection> {
    CachedSessionResources {
        shell: shells.remove(&session_id),
        sftp: sftps.remove(&session_id),
        connection: connections.remove(&session_id),
    }
}

fn remove_tunnel_for_session_rule<TTunnel>(
    tunnels: &mut HashMap<String, TTunnel>,
    session_id: SessionId,
    rule_name: &str,
) -> Option<TTunnel>
where
    TTunnel: TunnelOwner,
{
    if !tunnels
        .get(rule_name)
        .is_some_and(|tunnel| tunnel.session_id() == session_id)
    {
        return None;
    }

    tunnels.remove(rule_name)
}

trait TunnelOwner {
    fn session_id(&self) -> SessionId;
}

trait StoppableTunnel {
    fn stop(&self);
}

impl TunnelOwner for RemoteTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id()
    }
}

impl StoppableTunnel for RemoteTunnel {
    fn stop(&self) {
        RemoteTunnel::stop(self);
    }
}

fn replace_tunnel_stopping_previous<TTunnel>(
    tunnels: &mut HashMap<String, TTunnel>,
    tunnel: TTunnel,
) where
    TTunnel: RuleNamedTunnel + StoppableTunnel,
{
    if let Some(previous) = tunnels.insert(tunnel.rule_name().to_owned(), tunnel) {
        previous.stop();
    }
}

trait RuleNamedTunnel {
    fn rule_name(&self) -> &str;
}

impl RuleNamedTunnel for RemoteTunnel {
    fn rule_name(&self) -> &str {
        RemoteTunnel::rule_name(self)
    }
}

fn drop_cached_shell_after_failed_input<T>(
    shells: &mut HashMap<SessionId, T>,
    session_id: SessionId,
    result: &Result<(), BackendExecutionError>,
) -> bool {
    if !shell_input_result_requires_session_drop(result) {
        return false;
    }

    shells.remove(&session_id).is_some()
}

fn shell_input_result_requires_session_drop(result: &Result<(), BackendExecutionError>) -> bool {
    matches!(result, Err(BackendExecutionError::ChannelFailed { .. }))
}

fn drop_cached_sftp_after_failed_request<T>(
    sftps: &mut HashMap<SessionId, T>,
    session_id: SessionId,
    result: &Result<Vec<BackendEvent>, BackendExecutionError>,
) -> bool {
    if !sftp_result_requires_session_drop(result) {
        return false;
    }

    sftps.remove(&session_id).is_some()
}

fn sftp_result_requires_session_drop(
    result: &Result<Vec<BackendEvent>, BackendExecutionError>,
) -> bool {
    matches!(result, Err(BackendExecutionError::SftpFailed { .. }))
}
