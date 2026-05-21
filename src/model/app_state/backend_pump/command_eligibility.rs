//! 后端命令是否仍可执行的状态判定。

use crate::backend::{BackendCommand, SftpRequest};

use super::super::AppState;

impl AppState {
    pub(super) fn can_execute_backend_command(&self, command: &BackendCommand) -> bool {
        match command {
            BackendCommand::Connect { session_id, target } => self
                .sessions
                .can_execute_connect_command(*session_id, target.host_id),
            BackendCommand::SendShellInput { session_id, .. } => {
                self.sessions.can_send_interactive_shell_input(*session_id)
            }
            BackendCommand::DrainSessionOutput { session_id } => {
                self.sessions.can_drain_interactive_shell(*session_id)
            }
            BackendCommand::OpenShell { session_id, .. } => {
                self.sessions.can_execute_open_shell_command(*session_id)
            }
            BackendCommand::RunCommand { session_id, .. } => {
                self.sessions.can_execute_remote_command(*session_id)
            }
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::ListDir { .. },
            } => self.sessions.can_execute_sftp_browser_command(*session_id),
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::RemoveFile { .. } | SftpRequest::CreateDir { .. },
            } => self.sessions.can_execute_sftp_browser_command(*session_id),
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::Upload { .. } | SftpRequest::Download { .. },
            } => self.sessions.can_execute_sftp_transfer_command(*session_id),
            BackendCommand::StartTunnel {
                session_id,
                request,
            } => self
                .sessions
                .can_execute_tunnel_start_command(*session_id, &request.rule.name),
            BackendCommand::StopTunnel {
                session_id,
                request,
            } => self
                .sessions
                .can_execute_tunnel_stop_command(*session_id, &request.rule_name),
            _ => true,
        }
    }
}
