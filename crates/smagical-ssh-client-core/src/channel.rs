//! SSH channel 消息到后端事件的纯映射。

use russh::ChannelMsg;
use smagical_backend_core::{BackendEvent, BackendExecutionError};
use smagical_core::SessionId;
use smagical_terminal::TerminalSize;

use crate::HostKeyCheck;

/// SSH channel request 消息的纯状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelRequestStatus {
    Pending,
    Accepted,
}

/// 收集一次远程命令 channel 消息。
pub fn collect_command_message(
    session_id: SessionId,
    message: ChannelMsg,
    events: &mut Vec<BackendEvent>,
    exit_code: &mut Option<i32>,
) -> Result<bool, BackendExecutionError> {
    match message {
        ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
            events.push(output_event(session_id, data.as_ref()));
            Ok(false)
        }
        ChannelMsg::ExitStatus { exit_status } => {
            *exit_code = exit_status_to_i32(exit_status);
            Ok(false)
        }
        ChannelMsg::ExitSignal { signal_name, .. } => {
            events.push(BackendEvent::Output {
                session_id,
                line: format!("远程进程收到信号退出：{signal_name:?}"),
            });
            Ok(false)
        }
        ChannelMsg::Close => Ok(true),
        ChannelMsg::Failure => Err(BackendExecutionError::ChannelFailed {
            operation: "channel request".to_owned(),
            reason: "server rejected channel request".to_owned(),
        }),
        ChannelMsg::OpenFailure(reason) => Err(BackendExecutionError::ChannelFailed {
            operation: "channel open".to_owned(),
            reason: format!("{reason:?}"),
        }),
        _ => Ok(false),
    }
}

/// 收集一次 SSH channel request 响应消息。
pub fn collect_channel_request_message(
    operation: &str,
    message: ChannelMsg,
) -> Result<ChannelRequestStatus, BackendExecutionError> {
    match message {
        ChannelMsg::Success => Ok(ChannelRequestStatus::Accepted),
        ChannelMsg::Failure => Err(BackendExecutionError::ChannelFailed {
            operation: operation.to_owned(),
            reason: "server rejected channel request".to_owned(),
        }),
        ChannelMsg::OpenFailure(reason) => Err(BackendExecutionError::ChannelFailed {
            operation: operation.to_owned(),
            reason: format!("{reason:?}"),
        }),
        ChannelMsg::Close => Err(BackendExecutionError::ChannelFailed {
            operation: operation.to_owned(),
            reason: "channel closed before request succeeded".to_owned(),
        }),
        _ => Ok(ChannelRequestStatus::Pending),
    }
}

/// 创建 SSH channel request 等待结束错误。
pub fn channel_request_ended_error(operation: &str) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: "channel ended before request succeeded".to_owned(),
    }
}

/// 将交互式 shell channel 消息转换成后端事件。
pub fn shell_message_to_event(session_id: SessionId, message: ChannelMsg) -> Option<BackendEvent> {
    match message {
        ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
            Some(output_event(session_id, data.as_ref()))
        }
        ChannelMsg::ExitStatus { exit_status } => Some(command_exited_event(
            session_id,
            exit_status_to_i32(exit_status),
        )),
        ChannelMsg::Failure => Some(BackendEvent::Failed {
            session_id,
            reason: "server rejected channel request".to_owned(),
        }),
        ChannelMsg::OpenFailure(reason) => Some(BackendEvent::Failed {
            session_id,
            reason: format!("{reason:?}"),
        }),
        ChannelMsg::Close => Some(disconnected_event(session_id)),
        _ => None,
    }
}

/// 将 russh channel 错误转换成后端执行错误。
pub fn channel_error(operation: &str, error: russh::Error) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: error.to_string(),
    }
}

/// 创建 SSH 会话未连接错误。
pub fn connected_session_error(operation: &str) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: "session is not connected".to_owned(),
    }
}

/// 将 russh 连接错误转换成后端执行错误。
pub fn connection_error(endpoint: &str, error: russh::Error) -> BackendExecutionError {
    BackendExecutionError::ConnectionFailed {
        endpoint: endpoint.to_owned(),
        reason: error.to_string(),
    }
}

/// 将被拒绝的主机密钥校验结果转换成后端执行错误。
pub fn host_key_rejected_error(check: HostKeyCheck) -> BackendExecutionError {
    BackendExecutionError::HostKeyRejected {
        host: check.host,
        port: check.port,
        key_algorithm: check.key_algorithm,
        fingerprint: check.fingerprint,
        verification: check.verification,
    }
}

/// 创建终端输出事件。
pub fn output_event(session_id: SessionId, data: &[u8]) -> BackendEvent {
    BackendEvent::Output {
        session_id,
        line: String::from_utf8_lossy(data).into_owned(),
    }
}

/// 创建 SSH 连接开始事件。
pub fn connecting_event(session_id: SessionId, endpoint: String) -> BackendEvent {
    BackendEvent::Connecting {
        session_id,
        endpoint,
    }
}

/// 创建 SSH 主机密钥已校验事件。
pub fn host_key_verified_event(session_id: SessionId, check: HostKeyCheck) -> BackendEvent {
    BackendEvent::HostKeyVerified {
        session_id,
        host: check.host,
        port: check.port,
        key_algorithm: check.key_algorithm,
        fingerprint: check.fingerprint,
        result: check.verification,
    }
}

/// 创建 SSH 认证开始事件。
pub fn authenticating_event(session_id: SessionId, username: String) -> BackendEvent {
    BackendEvent::Authenticating {
        session_id,
        username,
    }
}

/// 创建 SSH 认证成功事件。
pub fn authenticated_event(session_id: SessionId) -> BackendEvent {
    BackendEvent::Authenticated { session_id }
}

/// 创建 SSH 连接可用事件。
pub fn connected_event(session_id: SessionId) -> BackendEvent {
    BackendEvent::Connected { session_id }
}

/// 创建远程 shell 已打开事件。
pub fn shell_opened_event(session_id: SessionId) -> BackendEvent {
    BackendEvent::ShellOpened { session_id }
}

/// 创建远程命令已开始事件。
pub fn remote_command_started_event(session_id: SessionId, command: String) -> BackendEvent {
    BackendEvent::RemoteCommandStarted {
        session_id,
        command,
    }
}

/// 创建命令退出事件。
pub fn command_exited_event(session_id: SessionId, exit_code: Option<i32>) -> BackendEvent {
    BackendEvent::CommandExited {
        session_id,
        exit_code,
    }
}

/// 创建 SSH 会话断开事件。
pub fn disconnected_event(session_id: SessionId) -> BackendEvent {
    BackendEvent::Disconnected { session_id }
}

/// 将 SSH 退出码转换成状态层可存储的退出码。
pub fn exit_status_to_i32(exit_status: u32) -> Option<i32> {
    i32::try_from(exit_status).ok()
}

/// 返回 SSH PTY 可接受的终端列数。
pub fn pty_columns(size: TerminalSize) -> u32 {
    u32::from(size.columns.max(1))
}

/// 返回 SSH PTY 可接受的终端行数。
pub fn pty_rows(size: TerminalSize) -> u32 {
    u32::from(size.rows.max(1))
}
