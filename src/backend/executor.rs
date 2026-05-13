//! 后端命令执行器抽象。

use std::collections::VecDeque;

use super::{BackendCommand, BackendCommandKind, BackendEvent};

/// 后端执行错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackendExecutionError {
    #[error("后端执行器不支持命令：{kind:?}")]
    UnsupportedCommand { kind: BackendCommandKind },
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
pub trait BackendExecutor {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError>;
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
mod tests {
    use super::*;
    use crate::backend::{ConnectionTarget, PtyRequest};
    use crate::model::{AuthProfile, Host, HostId, SessionId};
    use crate::terminal::TerminalSize;
    use uuid::Uuid;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    fn host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            tags: Vec::new(),
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                key_hint: None,
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn noop_executor_reports_unsupported_command() {
        let mut executor = NoopBackendExecutor;
        let session_id = session_id();

        let error = executor
            .execute(BackendCommand::OpenShell {
                session_id,
                pty: PtyRequest::xterm(TerminalSize::default()),
            })
            .expect_err("占位执行器应该拒绝命令");

        assert_eq!(
            error,
            BackendExecutionError::UnsupportedCommand {
                kind: BackendCommandKind::OpenShell
            }
        );
    }

    #[test]
    fn scripted_executor_returns_matching_events() {
        let mut executor = ScriptedBackendExecutor::new();
        let session_id = session_id();
        executor.push_response(ScriptedBackendResponse::new(
            BackendCommandKind::Connect,
            vec![BackendEvent::Connected { session_id }],
        ));

        let events = executor
            .execute(BackendCommand::Connect {
                session_id,
                target: ConnectionTarget::from_host(&host()),
            })
            .expect("脚本执行器应该返回匹配事件");

        assert_eq!(events, vec![BackendEvent::Connected { session_id }]);
        assert_eq!(executor.executed(), &[BackendCommandKind::Connect]);
        assert_eq!(executor.remaining(), 0);
    }

    #[test]
    fn scripted_executor_rejects_unexpected_command_kind() {
        let mut executor = ScriptedBackendExecutor::new();
        let session_id = session_id();
        executor.push_response(ScriptedBackendResponse::new(
            BackendCommandKind::Disconnect,
            Vec::new(),
        ));

        let error = executor
            .execute(BackendCommand::OpenShell {
                session_id,
                pty: PtyRequest::xterm(TerminalSize::default()),
            })
            .expect_err("命令类型不匹配应该失败");

        assert_eq!(
            error,
            BackendExecutionError::UnexpectedCommand {
                expected: BackendCommandKind::Disconnect,
                actual: BackendCommandKind::OpenShell,
            }
        );
        assert_eq!(executor.executed(), &[]);
        assert_eq!(executor.remaining(), 1);
    }

    #[test]
    fn scripted_executor_reports_missing_response_without_recording_command() {
        let mut executor = ScriptedBackendExecutor::new();
        let session_id = session_id();

        let error = executor
            .execute(BackendCommand::Disconnect { session_id })
            .expect_err("没有脚本响应时应该失败");

        assert_eq!(error, BackendExecutionError::NoScriptedResponse);
        assert_eq!(executor.executed(), &[]);
    }
}
