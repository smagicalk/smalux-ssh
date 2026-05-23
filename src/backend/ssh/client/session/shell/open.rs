//! 交互式远程 shell 打开流程。

use smagical_ssh_client_core::{
    OPEN_SHELL_OPERATION, OPEN_SHELL_SESSION_OPERATION, REQUEST_SHELL_OPERATION, channel_error,
    shell_opened_event,
};

use crate::backend::{BackendEvent, BackendExecutionError, PtyRequest};
use crate::model::SessionId;

use super::super::{open_session_channel, prepare_pty, wait_channel_request};
use super::RemoteShell;
use crate::backend::ssh::client::RusshConnection;

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
            shell: RemoteShell::new(session_id, channel),
            events: vec![shell_opened_event(session_id)],
        })
    }
}
