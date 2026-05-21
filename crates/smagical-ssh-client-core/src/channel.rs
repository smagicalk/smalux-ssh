//! SSH channel 消息到后端事件的纯映射。

use russh::ChannelMsg;
use smagical_backend_core::{BackendEvent, BackendExecutionError};
use smagical_core::SessionId;
use smagical_terminal::TerminalSize;

use crate::{HostKeyCheck, SharedHostKeyResult};

/// SSH channel request 消息的纯状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelRequestStatus {
    Pending,
    Accepted,
}

/// 打开交互式 shell 前的 session channel 操作名。
pub const OPEN_SHELL_SESSION_OPERATION: &str = "open shell session";

/// 请求交互式 shell 的操作名。
pub const REQUEST_SHELL_OPERATION: &str = "request shell";

/// 向远程 shell 写入输入的操作名。
pub const SHELL_INPUT_OPERATION: &str = "shell input";

/// 通知远程 shell 输入结束的操作名。
pub const SHELL_EOF_OPERATION: &str = "shell eof";

/// 调整远程 shell PTY 尺寸的操作名。
pub const SHELL_RESIZE_OPERATION: &str = "shell resize";

/// 执行远程命令前的 session channel 操作名。
pub const RUN_COMMAND_SESSION_OPERATION: &str = "run command session";

/// 请求执行远程命令的操作名。
pub const EXEC_COMMAND_OPERATION: &str = "exec command";

/// 执行器打开 shell 的前置会话操作名。
pub const OPEN_SHELL_OPERATION: &str = "open shell";

/// 执行器发送 shell 输入的前置会话操作名。
pub const SEND_SHELL_INPUT_OPERATION: &str = "send shell input";

/// 执行器运行远程命令的前置会话操作名。
pub const RUN_COMMAND_OPERATION: &str = "run command";

/// 执行器访问 SFTP 的前置会话操作名。
pub const SFTP_OPERATION: &str = "sftp";

/// 执行器启动隧道的前置会话操作名。
pub const START_TUNNEL_OPERATION: &str = "start tunnel";

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

/// 判断远程 shell drain 是否应停止继续轮询。
pub fn shell_drain_should_stop(event: &BackendEvent) -> bool {
    matches!(event, BackendEvent::Disconnected { .. })
}

/// 将 russh channel 错误转换成后端执行错误。
pub fn channel_error(operation: &str, error: russh::Error) -> BackendExecutionError {
    channel_reason_error(operation, error)
}

/// 将错误原因转换成后端 SSH channel 错误。
pub fn channel_reason_error(
    operation: &str,
    reason: impl std::fmt::Display,
) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: reason.to_string(),
    }
}

/// 创建 SSH 会话未连接错误。
pub fn connected_session_error(operation: &str) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: "session is not connected".to_owned(),
    }
}

/// 判断执行错误是否来自 SSH channel。
pub fn is_channel_failure(error: &BackendExecutionError) -> bool {
    matches!(error, BackendExecutionError::ChannelFailed { .. })
}

/// 返回 SSH channel 错误中的操作名和原因。
pub fn channel_failure_parts(error: &BackendExecutionError) -> Option<(&str, &str)> {
    match error {
        BackendExecutionError::ChannelFailed { operation, reason } => Some((operation, reason)),
        _ => None,
    }
}

/// 将 russh 连接错误转换成后端执行错误。
pub fn connection_error(endpoint: &str, error: russh::Error) -> BackendExecutionError {
    BackendExecutionError::ConnectionFailed {
        endpoint: endpoint.to_owned(),
        reason: error.to_string(),
    }
}

/// 根据 host key 校验结果优先返回主机密钥拒绝错误，否则返回连接错误。
pub fn host_key_or_connection_error(
    endpoint: &str,
    host_key_result: &SharedHostKeyResult,
    error: russh::Error,
) -> BackendExecutionError {
    if let Some(check) = host_key_result.get()
        && !check.accepted
    {
        return host_key_rejected_error(check);
    }

    connection_error(endpoint, error)
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
