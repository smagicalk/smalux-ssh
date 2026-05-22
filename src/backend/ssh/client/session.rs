//! SSH session channel、PTY、shell 和远程命令执行。

use crate::backend::{BackendEvent, BackendExecutionError, RemoteCommandRequest};
use crate::model::SessionId;

use super::RusshConnection;

mod channel;
mod command;
mod sftp;
mod shell;
mod tunnel;
use channel::{open_session_channel, prepare_pty, wait_channel_request};
pub use sftp::RemoteSftp;
pub use shell::{OpenShellReport, RemoteShell};
pub use tunnel::RemoteTunnel;

impl RusshConnection {
    /// 执行一次性远程命令并收集输出事件。
    pub async fn run_command(
        &mut self,
        session_id: SessionId,
        request: &RemoteCommandRequest,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        command::run_remote_command(self, session_id, request).await
    }
}
