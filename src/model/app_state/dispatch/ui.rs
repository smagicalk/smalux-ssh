//! UI 草稿与工作区界面消息路由。
//!
//! 这里处理“尚未发起真实 SSH/SFTP/存储动作”的界面交互：输入框草稿、弹窗、
//! 面板宽度、语言、主题选择、终端输入草稿等。它仍在核心状态层中，目的是让
//! 新 UI 能复用同一套交互状态和测试。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 分发纯 UI 状态消息。
    ///
    /// `MessageDispatchTarget` 已经保证只会把 UI 类消息送到这里；最后的
    /// `unreachable!` 用来捕获新增消息未分类或分类错误。
    pub(super) fn dispatch_ui_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::UpdateQuickHostDraft { field, value } => {
                self.update_quick_host_draft(field, value)
            }
            Message::SelectQuickHostGroup { group_id } => self.select_quick_host_group(group_id),
            Message::UpdateQuickHostAuthKind { kind } => self.update_quick_host_auth_kind(kind),
            Message::UpdateQuickHostAuthField { field, value } => {
                self.update_quick_host_auth_field(field, value)
            }
            Message::SaveQuickHost => self.save_quick_host(),
            Message::OpenCreateHostDialogInGroup { group_id } => {
                self.open_create_host_dialog_in_group(group_id)
            }
            Message::OpenCreateGroupParentDialog { parent_id } => {
                self.open_create_group_parent_dialog(parent_id)
            }
            Message::SelectCreateGroupParent { parent_id } => {
                self.select_create_group_parent(parent_id)
            }
            Message::CloseCreateGroupParentDialog => self.close_create_group_parent_dialog(),
            Message::ConfirmCreateGroupParent => self.confirm_create_group_parent(),
            Message::OpenCreateGroupDialog { parent_id } => {
                self.open_create_group_dialog(parent_id)
            }
            Message::UpdateQuickGroupName { name } => self.update_quick_group_name(name),
            Message::SelectQuickGroupParent { parent_id } => {
                self.select_quick_group_parent(parent_id)
            }
            Message::CloseCreateGroupDialog => self.close_create_group_dialog(),
            Message::SaveQuickGroup => self.save_quick_group(),
            Message::DismissUiError => self.dismiss_ui_error(),
            Message::OpenCreateHostDialog => self.open_create_host_dialog(),
            Message::OpenEditHostDialog { host_id } => self.open_edit_host_dialog(host_id),
            Message::DuplicateHost { host_id } => self.duplicate_host(host_id),
            Message::CloseCreateHostDialog => self.close_create_host_dialog(),
            Message::RequestRemoveHost { host_id } => self.request_remove_host(host_id),
            Message::CancelRemoveHost => self.cancel_remove_host(),
            Message::RequestRemoveGroup { group_id } => self.request_remove_group(group_id),
            Message::CancelRemoveGroup => self.cancel_remove_group(),
            Message::SetWorkspacePage { page } => self.set_workspace_page(page),
            Message::NavigateWorkspacePage { page } => self.navigate_workspace_page(page),
            Message::ToggleHostListMode => self.toggle_host_list_mode(),
            Message::ToggleHostTreeGroup { group_id } => self.toggle_host_tree_group(group_id),
            Message::UpdateHostSearchQuery { query } => self.update_host_search_query(query),
            Message::UpdateNewSessionSearchQuery { query } => {
                self.update_new_session_search_query(query)
            }
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
            Message::NextTheme => self.next_theme(),
            Message::SetLanguage { language } => self.set_language(language),
            Message::SetBuiltInTheme { theme } => self.set_built_in_theme(theme),
            Message::ExportCurrentTheme {
                target_path,
                format,
            } => self.export_current_theme(&target_path, format),
            Message::CopyCurrentBuiltInTheme => self.copy_current_built_in_theme(),
            Message::ImportTheme { source_path } => self.import_theme(&source_path),
            Message::ApplyThemeProfile { name } => self.apply_theme_profile(&name),
            Message::RemoveThemeProfile { name } => self.remove_theme_profile(&name),
            Message::BackupStorage { target_path } => self.backup_storage(&target_path),
            Message::ExportStorageSnapshot { target_path } => {
                self.export_storage_snapshot(&target_path)
            }
            Message::ImportStorageSnapshot { source_path } => {
                self.import_storage_snapshot(&source_path)
            }
            Message::ImportSqliteBackup { source_path } => self.import_sqlite_backup(&source_path),
            Message::NextBackground => self.next_background(),
            Message::OpenLocalTerminal => self.open_local_terminal(),
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
