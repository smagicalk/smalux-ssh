//! 后端命令执行器抽象。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use smagical_core::{HostKeyVerification, KeyAlgorithm};

use super::{BackendCommand, BackendCommandKind, BackendEvent};

/// 后端执行错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackendExecutionError {
    #[error("后端执行器不支持命令：{kind:?}")]
    UnsupportedCommand { kind: BackendCommandKind },
    #[error("连接 {endpoint} 失败：{reason}")]
    ConnectionFailed { endpoint: String, reason: String },
    #[error("主机密钥未被信任：{host}:{port} {fingerprint}")]
    HostKeyRejected {
        host: String,
        port: u16,
        key_algorithm: KeyAlgorithm,
        fingerprint: String,
        verification: HostKeyVerification,
    },
    #[error("用户 {username} 认证失败：{reason}")]
    AuthenticationFailed { username: String, reason: String },
    #[error("{operation} 通道失败：{reason}")]
    ChannelFailed { operation: String, reason: String },
    #[error("SFTP {operation} 失败：{reason}")]
    SftpFailed { operation: String, reason: String },
    #[error("隧道 {rule_name} 失败：{reason}")]
    TunnelFailed { rule_name: String, reason: String },
    #[error("脚本执行器期望 {expected:?}，实际收到 {actual:?}")]
    UnexpectedCommand {
        expected: BackendCommandKind,
        actual: BackendCommandKind,
    },
    #[error("脚本执行器没有剩余响应")]
    NoScriptedResponse,
}

/// 后端命令执行器。
///
/// 真实 SSH 后端可以在实现中启动异步任务，把后续事件通过通道送回 UI；
/// 测试后端可以直接返回一组确定事件。
pub trait BackendExecutor: Send {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError>;
}

/// 共享后端执行器句柄。
pub type SharedBackendExecutor = Arc<Mutex<Box<dyn BackendExecutor>>>;

/// 把一个执行器包装成共享句柄。
pub fn shared_backend_executor<E>(executor: E) -> SharedBackendExecutor
where
    E: BackendExecutor + 'static,
{
    Arc::new(Mutex::new(Box::new(executor)))
}

/// 创建占位共享执行器。
pub fn noop_shared_backend_executor() -> SharedBackendExecutor {
    shared_backend_executor(NoopBackendExecutor)
}

/// 不执行任何命令的占位执行器。
#[derive(Debug, Clone, Default)]
pub struct NoopBackendExecutor;

impl BackendExecutor for NoopBackendExecutor {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        Err(BackendExecutionError::UnsupportedCommand {
            kind: command.kind(),
        })
    }
}

/// 脚本化执行响应。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedBackendResponse {
    pub expected: BackendCommandKind,
    pub events: Vec<BackendEvent>,
}

impl ScriptedBackendResponse {
    /// 创建一条脚本响应。
    pub fn new(expected: BackendCommandKind, events: Vec<BackendEvent>) -> Self {
        Self { expected, events }
    }
}

/// 用于单元测试和 UI 联调的脚本执行器。
#[derive(Debug, Clone, Default)]
pub struct ScriptedBackendExecutor {
    responses: VecDeque<ScriptedBackendResponse>,
    executed: Vec<BackendCommandKind>,
}

impl ScriptedBackendExecutor {
    /// 创建空脚本执行器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一条脚本响应。
    pub fn push_response(&mut self, response: ScriptedBackendResponse) {
        self.responses.push_back(response);
    }

    /// 返回已经执行过的命令类型。
    pub fn executed(&self) -> &[BackendCommandKind] {
        &self.executed
    }

    /// 返回剩余脚本响应数量。
    pub fn remaining(&self) -> usize {
        self.responses.len()
    }
}

impl BackendExecutor for ScriptedBackendExecutor {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let actual = command.kind();
        let response = self
            .responses
            .front()
            .ok_or(BackendExecutionError::NoScriptedResponse)?;

        if response.expected != actual {
            return Err(BackendExecutionError::UnexpectedCommand {
                expected: response.expected,
                actual,
            });
        }

        let response = self.responses.pop_front().expect("front 已确认存在响应");
        self.executed.push(actual);
        Ok(response.events)
    }
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
