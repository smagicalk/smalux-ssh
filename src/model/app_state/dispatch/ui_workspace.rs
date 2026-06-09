//! 工作区页面、搜索、设置页和通用 UI 消息。

use super::super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn try_dispatch_workspace_ui_message(
        &mut self,
        message: &Message,
    ) -> Option<AppUpdateOutcome> {
        Some(match message {
            Message::DismissUiError => self.dismiss_ui_error(),
            Message::SetWorkspacePage { page } => self.set_workspace_page(page.clone()),
            Message::NavigateWorkspacePage { page } => self.navigate_workspace_page(page.clone()),
            Message::ToggleHostListMode => self.toggle_host_list_mode(),
            Message::ToggleHostTreeGroup { group_id } => self.toggle_host_tree_group(*group_id),
            Message::ToggleCredentialTreeNode { node_id } => {
                self.toggle_credential_tree_node(node_id.clone())
            }
            Message::UpdateHostSearchQuery { query } => {
                self.update_host_search_query(query.clone())
            }
            Message::UpdateCredentialSearchQuery { query } => {
                self.update_credential_search_query(query.clone())
            }
            Message::UpdateSnippetSearchQuery { query } => {
                self.update_snippet_search_query(query.clone())
            }
            Message::ToggleSnippetTreeNode { node_id } => {
                self.toggle_snippet_tree_node(node_id.clone())
            }
            Message::UpdateNewSessionSearchQuery { query } => {
                self.update_new_session_search_query(query.clone())
            }
            Message::ResizeHostsPanel { width } => self.resize_hosts_panel(*width),
            Message::ResizeActivityPanel { width } => self.resize_activity_panel(*width),
            Message::ResizeToolPanel { width } => self.resize_tool_panel(*width),
            Message::OpenToolPanel { mode } => self.open_tool_panel(mode.clone()),
            Message::CloseToolPanel => self.close_tool_panel(),
            Message::ToggleRightSidebar => self.toggle_right_sidebar(),
            Message::OpenCommandPalette { query } => self.open_command_palette(query.clone()),
            Message::UpdateCommandPaletteQuery { query } => {
                self.update_command_palette_query(query.clone())
            }
            Message::CloseCommandPalette => self.close_command_palette(),
            Message::NextTheme => self.next_theme(),
            Message::SetLanguage { language } => self.set_language(language.clone()),
            Message::SetBuiltInTheme { theme } => self.set_built_in_theme(*theme),
            Message::ExportCurrentTheme {
                target_path,
                format,
            } => self.export_current_theme(target_path, format.clone()),
            Message::CopyCurrentBuiltInTheme => self.copy_current_built_in_theme(),
            Message::ImportTheme { source_path } => self.import_theme(source_path),
            Message::ApplyThemeProfile { name } => self.apply_theme_profile(name),
            Message::RemoveThemeProfile { name } => self.remove_theme_profile(name),
            Message::BackupStorage { target_path } => self.backup_storage(target_path),
            Message::ExportStorageSnapshot { target_path } => {
                self.export_storage_snapshot(target_path)
            }
            Message::ImportStorageSnapshot { source_path } => {
                self.import_storage_snapshot(source_path)
            }
            Message::ImportSqliteBackup { source_path } => self.import_sqlite_backup(source_path),
            Message::NextBackground => self.next_background(),
            _ => return None,
        })
    }
}
