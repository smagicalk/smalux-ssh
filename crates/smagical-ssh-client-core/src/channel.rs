//! SSH channel 消息到后端事件的纯映射。

use russh::ChannelMsg;
use smagical_backend_core::{BackendEvent, BackendExecutionError};
use smagical_core::SessionId;
use smagical_terminal::TerminalSize;

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

/// 将交互式 shell channel 消息转换成后端事件。
pub fn shell_message_to_event(session_id: SessionId, message: ChannelMsg) -> Option<BackendEvent> {
    match message {
        ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
            Some(output_event(session_id, data.as_ref()))
        }
        ChannelMsg::ExitStatus { exit_status } => Some(BackendEvent::CommandExited {
            session_id,
            exit_code: exit_status_to_i32(exit_status),
        }),
        ChannelMsg::Failure => Some(BackendEvent::Failed {
            session_id,
            reason: "server rejected channel request".to_owned(),
        }),
        ChannelMsg::OpenFailure(reason) => Some(BackendEvent::Failed {
            session_id,
            reason: format!("{reason:?}"),
        }),
        ChannelMsg::Close => Some(BackendEvent::Disconnected { session_id }),
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

/// 创建终端输出事件。
pub fn output_event(session_id: SessionId, data: &[u8]) -> BackendEvent {
    BackendEvent::Output {
        session_id,
        line: String::from_utf8_lossy(data).into_owned(),
    }
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
