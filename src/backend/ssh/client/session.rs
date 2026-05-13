//! SSH session channel、PTY、shell 和远程命令执行。

use std::io::Cursor;

use russh::client;
use russh::{Channel, ChannelMsg};

use crate::backend::{BackendEvent, BackendExecutionError, PtyRequest, RemoteCommandRequest};
use crate::model::SessionId;
use crate::terminal::TerminalSize;

use super::RusshConnection;

#[cfg(test)]
mod tests;

/// 已打开的交互式远程 shell。
pub struct RemoteShell {
    session_id: SessionId,
    channel: Channel<client::Msg>,
}

impl RemoteShell {
    /// 返回 shell 关联的会话标识。
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// 向远程 shell 写入用户输入。
    pub async fn send_input(&self, input: &[u8]) -> Result<(), BackendExecutionError> {
        self.channel
            .data(Cursor::new(input.to_vec()))
            .await
            .map_err(|error| channel_error("shell input", error))
    }

    /// 通知远程 shell 本地输入已经结束。
    pub async fn close_input(&self) -> Result<(), BackendExecutionError> {
        self.channel
            .eof()
            .await
            .map_err(|error| channel_error("shell eof", error))
    }

    /// 同步终端窗口尺寸。
    pub async fn resize(&self, size: TerminalSize) -> Result<(), BackendExecutionError> {
        self.channel
            .window_change(columns(size), rows(size), 0, 0)
            .await
            .map_err(|error| channel_error("shell resize", error))
    }

    /// 读取下一条远程 shell 输出事件。
    pub async fn next_event(&mut self) -> Option<BackendEvent> {
        match self.channel.wait().await {
            Some(message) => shell_message_to_event(self.session_id, message),
            None => Some(BackendEvent::Disconnected {
                session_id: self.session_id,
            }),
        }
    }
}

/// 打开远程 shell 后返回的事件和可交互句柄。
pub struct OpenShellReport {
    pub shell: RemoteShell,
    pub events: Vec<BackendEvent>,
}

impl RusshConnection {
    /// 打开交互式远程 shell。
    pub async fn open_shell(
        &mut self,
        session_id: SessionId,
        pty: &PtyRequest,
    ) -> Result<OpenShellReport, BackendExecutionError> {
        let mut channel = self.open_session_channel("open shell session").await?;
        prepare_pty(&mut channel, pty, "open shell").await?;
        channel
            .request_shell(true)
            .await
            .map_err(|error| channel_error("request shell", error))?;
        wait_channel_request(&mut channel, "request shell").await?;

        Ok(OpenShellReport {
            shell: RemoteShell {
                session_id,
                channel,
            },
            events: vec![BackendEvent::ShellOpened { session_id }],
        })
    }

    /// 执行一次性远程命令并收集输出事件。
    pub async fn run_command(
        &mut self,
        session_id: SessionId,
        request: &RemoteCommandRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let mut channel = self.open_session_channel("run command session").await?;
        if let Some(pty) = &request.pty {
            prepare_pty(&mut channel, pty, "run command").await?;
        }

        channel
            .exec(true, request.command.clone())
            .await
            .map_err(|error| channel_error("exec command", error))?;
        wait_channel_request(&mut channel, "exec command").await?;

        let mut events = vec![BackendEvent::RemoteCommandStarted {
            session_id,
            command: request.command.clone(),
        }];
        let mut exit_code = None;

        while let Some(message) = channel.wait().await {
            if collect_command_message(session_id, message, &mut events, &mut exit_code)? {
                break;
            }
        }

        events.push(BackendEvent::CommandExited {
            session_id,
            exit_code,
        });

        Ok(events)
    }

    async fn open_session_channel(
        &mut self,
        operation: &str,
    ) -> Result<Channel<client::Msg>, BackendExecutionError> {
        self.handle_mut()
            .channel_open_session()
            .await
            .map_err(|error| channel_error(operation, error))
    }
}

async fn prepare_pty(
    channel: &mut Channel<client::Msg>,
    pty: &PtyRequest,
    operation: &str,
) -> Result<(), BackendExecutionError> {
    for (name, value) in &pty.environment {
        channel
            .set_env(false, name.clone(), value.clone())
            .await
            .map_err(|error| channel_error(operation, error))?;
    }

    channel
        .request_pty(
            true,
            &pty.term,
            columns(pty.size),
            rows(pty.size),
            0,
            0,
            &[],
        )
        .await
        .map_err(|error| channel_error(operation, error))?;
    wait_channel_request(channel, operation).await
}

async fn wait_channel_request(
    channel: &mut Channel<client::Msg>,
    operation: &str,
) -> Result<(), BackendExecutionError> {
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Success => return Ok(()),
            ChannelMsg::Failure => {
                return Err(BackendExecutionError::ChannelFailed {
                    operation: operation.to_owned(),
                    reason: "server rejected channel request".to_owned(),
                });
            }
            ChannelMsg::OpenFailure(reason) => {
                return Err(BackendExecutionError::ChannelFailed {
                    operation: operation.to_owned(),
                    reason: format!("{reason:?}"),
                });
            }
            ChannelMsg::Close => {
                return Err(BackendExecutionError::ChannelFailed {
                    operation: operation.to_owned(),
                    reason: "channel closed before request succeeded".to_owned(),
                });
            }
            _ => {}
        }
    }

    Err(BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: "channel ended before request succeeded".to_owned(),
    })
}

fn collect_command_message(
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

fn shell_message_to_event(session_id: SessionId, message: ChannelMsg) -> Option<BackendEvent> {
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

fn output_event(session_id: SessionId, data: &[u8]) -> BackendEvent {
    BackendEvent::Output {
        session_id,
        line: String::from_utf8_lossy(data).into_owned(),
    }
}

fn exit_status_to_i32(exit_status: u32) -> Option<i32> {
    i32::try_from(exit_status).ok()
}

fn columns(size: TerminalSize) -> u32 {
    u32::from(size.columns.max(1))
}

fn rows(size: TerminalSize) -> u32 {
    u32::from(size.rows.max(1))
}

fn channel_error(operation: &str, error: russh::Error) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: error.to_string(),
    }
}
