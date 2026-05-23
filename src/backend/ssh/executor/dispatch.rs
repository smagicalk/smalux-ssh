//! SSH executor 命令分发。

use crate::backend::{BackendCommand, BackendEvent, BackendExecutionError, BackendExecutor};
use crate::security::SecretStore;

use super::RusshBackendExecutor;

impl<S: SecretStore + Send> BackendExecutor for RusshBackendExecutor<S> {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        match command {
            BackendCommand::Connect { session_id, target } => self.connect(session_id, target),
            BackendCommand::OpenShell { session_id, pty } => self.open_shell(session_id, pty),
            BackendCommand::RunCommand {
                session_id,
                request,
            } => self.run_command(session_id, request),
            BackendCommand::SendShellInput { session_id, input } => {
                self.send_shell_input(session_id, input)
            }
            BackendCommand::DrainSessionOutput { session_id } => {
                self.drain_session_output(session_id)
            }
            BackendCommand::Sftp {
                session_id,
                request,
            } => self.sftp(session_id, request),
            BackendCommand::StartTunnel {
                session_id,
                request,
            } => self.start_tunnel(session_id, request),
            BackendCommand::StopTunnel {
                session_id,
                request,
            } => self.stop_tunnel(session_id, request),
            BackendCommand::Disconnect { session_id } => self.disconnect(session_id),
        }
    }
}
