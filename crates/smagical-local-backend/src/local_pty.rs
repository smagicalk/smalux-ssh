//! 本地 PTY 后端执行器。
//!
//! 这个模块只负责本地 shell 进程和 PTY 读写，不读取 UI 状态，也不处理远程 SSH。

mod fallback;
mod reader;
mod session;

use std::collections::HashMap;

use smagical_backend_core::{
    BackendCommand, BackendEvent, BackendExecutionError, BackendExecutor, LocalShellProfile,
};
use smagical_core::SessionId;

use self::session::LocalPtySession;

pub(crate) use self::reader::terminal_event_to_backend;

/// 同时承载本地 PTY 和远程后端的组合执行器。
pub struct DesktopBackendExecutor<R> {
    /// 本地 shell 执行器。
    local: LocalPtyBackendExecutor,
    /// 远程 SSH/SFTP/隧道执行器。
    remote: R,
}

impl<R> DesktopBackendExecutor<R> {
    /// 创建桌面端默认后端执行器。
    pub fn new(remote: R) -> Self {
        Self {
            local: LocalPtyBackendExecutor::default(),
            remote,
        }
    }
}

impl<R> BackendExecutor for DesktopBackendExecutor<R>
where
    R: BackendExecutor,
{
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        // 本地会话一旦创建，后续输入和 drain 都必须继续路由到本地执行器。
        if matches!(command, BackendCommand::OpenLocalShell { .. })
            || self.local.has_session(command.session_id())
        {
            self.local.execute(command)
        } else {
            self.remote.execute(command)
        }
    }
}

/// 本地 PTY 后端执行器。
pub struct LocalPtyBackendExecutor {
    /// 当前存活的 PTY 会话。
    sessions: HashMap<SessionId, LocalPtySession>,
    /// 平台默认 shell profile。
    shell: LocalShellProfile,
}

impl Default for LocalPtyBackendExecutor {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            shell: LocalShellProfile::default_for_platform(),
        }
    }
}

impl LocalPtyBackendExecutor {
    /// 当前持有的本地 PTY 会话数量。
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// 判断指定会话是否由本地 PTY 执行器持有。
    pub fn has_session(&self, session_id: SessionId) -> bool {
        self.sessions.contains_key(&session_id)
    }

    fn ensure_session(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        if self.sessions.contains_key(&session_id) {
            // 重复打开同一会话时不再 spawn，只抽取已有输出。
            return Ok(self.drain_output(session_id));
        }

        // 成功 spawn 后立即发 Connected + ShellOpened，保持和远程 shell 的事件语义一致。
        let mut session = LocalPtySession::spawn(session_id, &self.shell)?;
        let mut events = vec![
            BackendEvent::Connected { session_id },
            BackendEvent::ShellOpened { session_id },
        ];
        events.extend(session.drain_output());
        self.sessions.insert(session_id, session);
        Ok(events)
    }

    fn send_input(
        &mut self,
        session_id: SessionId,
        input: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let startup_events = self.ensure_session(session_id)?;
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| local_pty_error("send input", "local session missing after start"))?;

        let mut events = startup_events;
        session.write_input(&input)?;
        // fallback 用于某些平台读不到真实回显时补偿显示。
        session.remember_fallback(input);
        events.extend(session.drain_output());

        Ok(events)
    }

    fn disconnect(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        // 移除 session 会释放 LocalPtySession，Drop 负责清理底层进程/句柄。
        self.sessions.remove(&session_id);
        Ok(vec![BackendEvent::Disconnected { session_id }])
    }

    fn drain_output(&mut self, session_id: SessionId) -> Vec<BackendEvent> {
        let Some(session) = self.sessions.get_mut(&session_id) else {
            return Vec::new();
        };

        session.drain_output()
    }
}

impl BackendExecutor for LocalPtyBackendExecutor {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        match command {
            BackendCommand::OpenLocalShell { session_id, .. } => self.ensure_session(session_id),
            BackendCommand::OpenShell { session_id, .. } => self.ensure_session(session_id),
            BackendCommand::SendShellInput { session_id, input } => {
                self.send_input(session_id, input)
            }
            BackendCommand::DrainSessionOutput { session_id } => Ok(self.drain_output(session_id)),
            BackendCommand::Disconnect { session_id } => self.disconnect(session_id),
            other => Err(local_pty_error(
                "local pty",
                &format!("unsupported command: {:?}", other.kind()),
            )),
        }
    }
}

fn local_pty_error(operation: &str, reason: &str) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests;
