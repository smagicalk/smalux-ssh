//! SSH session 的交互式 shell。

use std::io::Cursor;
use std::time::Duration;

use russh::Channel;
use russh::client;
use smagical_ssh_client_core::{
    OPEN_SHELL_OPERATION, OPEN_SHELL_SESSION_OPERATION, REQUEST_SHELL_OPERATION,
    SHELL_EOF_OPERATION, SHELL_INPUT_OPERATION, SHELL_RESIZE_OPERATION, channel_error,
    disconnected_event, pty_columns, pty_rows, shell_drain_should_stop, shell_message_to_event,
    shell_opened_event,
};
use tokio::time::timeout;

use crate::backend::{BackendEvent, BackendExecutionError, PtyRequest};
use crate::model::SessionId;
use crate::terminal::TerminalSize;

use super::super::RusshConnection;
use super::{open_session_channel, prepare_pty, wait_channel_request};

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
            .map_err(|error| channel_error(SHELL_INPUT_OPERATION, error))
    }

    /// 通知远程 shell 本地输入已经结束。
    pub async fn close_input(&self) -> Result<(), BackendExecutionError> {
        self.channel
            .eof()
            .await
            .map_err(|error| channel_error(SHELL_EOF_OPERATION, error))
    }

    /// 同步终端窗口尺寸。
    pub async fn resize(&self, size: TerminalSize) -> Result<(), BackendExecutionError> {
        self.channel
            .window_change(pty_columns(size), pty_rows(size), 0, 0)
            .await
            .map_err(|error| channel_error(SHELL_RESIZE_OPERATION, error))
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
                events.push(disconnected_event(self.session_id));
                break;
            };

            let Some(event) = shell_message_to_event(self.session_id, message) else {
                continue;
            };

            let should_stop = shell_drain_should_stop(&event);
            events.push(event);
            if should_stop {
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
        let mut channel = open_session_channel(self, OPEN_SHELL_SESSION_OPERATION).await?;
        prepare_pty(&mut channel, pty, OPEN_SHELL_OPERATION).await?;
        channel
            .request_shell(true)
            .await
            .map_err(|error| channel_error(REQUEST_SHELL_OPERATION, error))?;
        wait_channel_request(&mut channel, REQUEST_SHELL_OPERATION).await?;

        Ok(OpenShellReport {
            shell: RemoteShell {
                session_id,
                channel,
            },
            events: vec![shell_opened_event(session_id)],
        })
    }
}
