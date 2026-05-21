use super::*;

#[path = "sftp_error.rs"]
mod error;
#[path = "sftp_terminal.rs"]
mod terminal;
#[path = "sftp_transfer.rs"]
mod transfer;

#[derive(Debug, Clone, Copy)]
struct FailingSftpExecutor;

impl BackendExecutor for FailingSftpExecutor {
    fn execute(
        &mut self,
        command: crate::backend::BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        assert!(matches!(
            command,
            crate::backend::BackendCommand::Sftp { .. }
        ));

        Err(BackendExecutionError::SftpFailed {
            operation: "list dir".to_owned(),
            reason: "permission denied".to_owned(),
        })
    }
}
