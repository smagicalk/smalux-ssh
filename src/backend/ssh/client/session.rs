//! SSH session channel、PTY、shell 和远程命令执行。

use std::io::Cursor;
use std::time::Duration;

use russh::client;
use russh::{Channel, ChannelMsg};
use tokio::time::timeout;

use crate::backend::{BackendEvent, BackendExecutionError, PtyRequest, RemoteCommandRequest};
use crate::model::SessionId;
use crate::terminal::TerminalSize;
use smagical_ssh_client_core::{collect_command_message, shell_message_to_event};

use super::RusshConnection;

mod sftp;
mod tunnel;
pub use sftp::RemoteSftp;
pub use tunnel::RemoteTunnel;

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

    /// 在给定时间预算内尽量抽干已经到达的远程 shell 输出。
    pub async fn drain_ready_events(
        &mut self,
        max_events: usize,
        poll_timeout: Duration,
    ) -> Vec<BackendEvent> {
        let mut events = Vec::new();

        if max_events == 0 {
            return events;
        }

        while events.len() < max_events {
            let message = if events.is_empty() {
                match timeout(poll_timeout, self.channel.wait()).await {
                    Ok(message) => message,
                    Err(_) => break,
                }
            } else {
                match timeout(Duration::ZERO, self.channel.wait()).await {
                    Ok(message) => message,
                    Err(_) => break,
                }
            };

            let Some(message) = message else {
                events.push(BackendEvent::Disconnected {
                    session_id: self.session_id,
                });
                break;
            };

            let Some(event) = shell_message_to_event(self.session_id, message) else {
                continue;
            };

            let disconnected = matches!(event, BackendEvent::Disconnected { .. });
            events.push(event);
            if disconnected {
                break;
            }
        }

        events
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

pub(super) async fn wait_channel_request(
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

fn columns(size: TerminalSize) -> u32 {
    u32::from(size.columns.max(1))
}

fn rows(size: TerminalSize) -> u32 {
    u32::from(size.rows.max(1))
}

pub(super) fn channel_error(operation: &str, error: russh::Error) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: error.to_string(),
    }
}
