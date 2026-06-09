//! 本地终端、输入草稿和主机动作草稿 UI 消息。

use super::super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn try_dispatch_terminal_ui_message(
        &mut self,
        message: &Message,
    ) -> Option<AppUpdateOutcome> {
        Some(match message {
            Message::OpenLocalTerminal => self.open_local_terminal(),
            Message::UpdateTerminalInputDraft { session_id, input } => {
                self.update_terminal_input_draft(*session_id, input.clone())
            }
            Message::AppendTerminalInputDraft { session_id, text } => {
                self.append_terminal_input_draft(*session_id, text.clone())
            }
            Message::BackspaceTerminalInputDraft { session_id } => {
                self.backspace_terminal_input_draft(*session_id)
            }
            Message::SendTerminalInput { session_id } => self.send_terminal_input(*session_id),
            Message::UpdateHostCommandDraft { host_id, command } => {
                self.update_host_command_draft(*host_id, command.clone())
            }
            Message::UpdateHostSftpInitialDirDraft {
                host_id,
                initial_dir,
            } => self.update_host_sftp_initial_dir_draft(*host_id, initial_dir.clone()),
            Message::UpdateSftpActionDraft {
                host_id,
                field,
                value,
            } => self.update_sftp_action_draft(*host_id, field.clone(), value.clone()),
            _ => return None,
        })
    }
}
