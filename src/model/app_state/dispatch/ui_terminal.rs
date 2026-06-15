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
                self.ui.set_terminal_input(*session_id, input.clone());
                draft_changed()
            }
            Message::AppendTerminalInputDraft { session_id, text } => {
                let filtered = printable_terminal_input(text);
                if filtered.is_empty() {
                    AppUpdateOutcome::default()
                } else {
                    self.ui.append_terminal_input(*session_id, filtered);
                    draft_changed()
                }
            }
            Message::BackspaceTerminalInputDraft { session_id } => {
                let before = self.ui.terminal_input_for(*session_id).to_owned();
                self.ui.backspace_terminal_input(*session_id);
                AppUpdateOutcome {
                    state_changed: before != self.ui.terminal_input_for(*session_id),
                    ..AppUpdateOutcome::default()
                }
            }
            Message::SendTerminalInput { session_id } => {
                let input = self.ui.terminal_input_for(*session_id).to_owned();
                let outcome = self.core.send_terminal_input(*session_id, input);
                if outcome.error.is_none() && outcome.state_changed {
                    self.ui.clear_terminal_input(*session_id);
                }
                outcome
            }
            Message::UpdateHostCommandDraft { host_id, command } => {
                self.ui.set_remote_command(*host_id, command.clone());
                draft_changed()
            }
            Message::UpdateHostSftpInitialDirDraft {
                host_id,
                initial_dir,
            } => {
                self.ui.set_sftp_initial_dir(*host_id, initial_dir.clone());
                draft_changed()
            }
            Message::UpdateSftpActionDraft {
                host_id,
                field,
                value,
            } => {
                self.ui
                    .set_sftp_action_field(*host_id, field.clone(), value.clone());
                draft_changed()
            }
            _ => return None,
        })
    }
}

fn draft_changed() -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}

fn printable_terminal_input(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control() && !('\u{e000}'..='\u{f8ff}').contains(ch))
        .collect()
}
