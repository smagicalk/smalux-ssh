//! UI 草稿与工作区界面消息路由。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn dispatch_ui_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::UpdateQuickHostDraft { field, value } => {
                self.update_quick_host_draft(field, value)
            }
            Message::UpdateQuickHostAuthKind { kind } => self.update_quick_host_auth_kind(kind),
            Message::UpdateQuickHostAuthField { field, value } => {
                self.update_quick_host_auth_field(field, value)
            }
            Message::SaveQuickHost => self.save_quick_host(),
            Message::DismissUiError => self.dismiss_ui_error(),
            Message::SetWorkspacePage { page } => self.set_workspace_page(page),
            Message::ToggleHostListMode => self.toggle_host_list_mode(),
            Message::UpdateHostSearchQuery { query } => self.update_host_search_query(query),
            Message::ResizeHostsPanel { width } => self.resize_hosts_panel(width),
            Message::ResizeActivityPanel { width } => self.resize_activity_panel(width),
            Message::ResizeToolPanel { width } => self.resize_tool_panel(width),
            Message::OpenToolPanel { mode } => self.open_tool_panel(mode),
            Message::CloseToolPanel => self.close_tool_panel(),
            Message::ToggleRightSidebar => self.toggle_right_sidebar(),
            Message::OpenCommandPalette { query } => self.open_command_palette(query),
            Message::UpdateCommandPaletteQuery { query } => {
                self.update_command_palette_query(query)
            }
            Message::CloseCommandPalette => self.close_command_palette(),
            Message::NextBackground => self.next_background(),
            Message::UpdateTerminalInputDraft { session_id, input } => {
                self.update_terminal_input_draft(session_id, input)
            }
            Message::AppendTerminalInputDraft { session_id, text } => {
                self.append_terminal_input_draft(session_id, text)
            }
            Message::BackspaceTerminalInputDraft { session_id } => {
                self.backspace_terminal_input_draft(session_id)
            }
            Message::SendTerminalInput { session_id } => self.send_terminal_input(session_id),
            Message::UpdateHostCommandDraft { host_id, command } => {
                self.update_host_command_draft(host_id, command)
            }
            Message::UpdateHostSftpInitialDirDraft {
                host_id,
                initial_dir,
            } => self.update_host_sftp_initial_dir_draft(host_id, initial_dir),
            Message::UpdateSftpActionDraft {
                host_id,
                field,
                value,
            } => self.update_sftp_action_draft(host_id, field, value),
            _ => unreachable!("非 UI 消息不应进入 UI 路由"),
        }
    }
}
