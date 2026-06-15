//! 工作区页面、搜索、设置页和通用 UI 消息。

use crate::config::HostListModePreference;
use crate::model::{HostListMode, ToolPanelMode, WorkspacePage};

use super::super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn try_dispatch_workspace_ui_message(
        &mut self,
        message: &Message,
    ) -> Option<AppUpdateOutcome> {
        Some(match message {
            Message::DismissUiError => self.dismiss_ui_error(),
            Message::SetWorkspacePage { page } => {
                self.ui.workspace.active_page = *page;
                self.ui.workspace.set_hosts_panel_collapsed(false);
                draft_changed()
            }
            Message::NavigateWorkspacePage { page } => {
                if self.ui.workspace.active_page == *page {
                    let collapsed = !self.ui.workspace.hosts_panel_collapsed;
                    self.ui.workspace.set_hosts_panel_collapsed(collapsed);
                } else {
                    self.ui.workspace.active_page = *page;
                    self.ui.workspace.set_hosts_panel_collapsed(false);
                }
                draft_changed()
            }
            Message::ToggleHostListMode => {
                self.ui.workspace.toggle_host_list_mode();
                let preference = match self.ui.workspace.host_list_mode {
                    HostListMode::Tree => HostListModePreference::Tree,
                    HostListMode::Card => HostListModePreference::Card,
                };
                let changed = self.config.workspace.host_list_mode != preference;
                self.config.workspace.host_list_mode = preference;
                self.storage.app_config = self.config.clone();
                AppUpdateOutcome {
                    state_changed: changed,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::ToggleHostTreeGroup { group_id } => {
                self.ui.workspace.toggle_host_tree_group(*group_id);
                draft_changed()
            }
            Message::ToggleCredentialTreeNode { node_id } => {
                self.ui
                    .workspace
                    .toggle_credential_tree_node(node_id.clone());
                draft_changed()
            }
            Message::UpdateHostSearchQuery { query } => {
                self.ui.workspace.set_host_search_query(query.clone());
                draft_changed()
            }
            Message::UpdateCredentialSearchQuery { query } => {
                self.ui.workspace.set_credential_search_query(query.clone());
                draft_changed()
            }
            Message::UpdateSnippetSearchQuery { query } => {
                self.ui.workspace.set_snippet_search_query(query.clone());
                draft_changed()
            }
            Message::UpdateNetworkSearchQuery { query } => {
                self.ui.workspace.set_network_search_query(query.clone());
                draft_changed()
            }
            Message::ToggleSnippetTreeNode { node_id } => {
                self.ui.workspace.toggle_snippet_tree_node(node_id.clone());
                draft_changed()
            }
            Message::UpdateNewSessionSearchQuery { query } => {
                self.ui
                    .workspace
                    .set_new_session_search_query(query.clone());
                draft_changed()
            }
            Message::ResizeHostsPanel { width } => {
                let before = self.ui.workspace.hosts_panel_width;
                self.ui.workspace.set_hosts_panel_width(*width);
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.hosts_panel_width,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::ResizeActivityPanel { width } => {
                let before = self.ui.workspace.activity_panel_width;
                self.ui.workspace.set_activity_panel_width(*width);
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.activity_panel_width,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::ResizeToolPanel { width } => {
                let before = self.ui.workspace.tool_panel_width;
                self.ui.workspace.set_tool_panel_width(*width);
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.tool_panel_width,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::OpenToolPanel { mode } => {
                let before = self.ui.workspace.tool_panel_mode;
                let before_page = self.ui.workspace.active_page;
                let before_active_tab = self.sessions.active_tab;
                self.ui.workspace.open_tool_panel(*mode);
                if matches!(mode, ToolPanelMode::Sftp) {
                    self.ui.workspace.active_page = WorkspacePage::Terminal;
                    if let Some(active_terminal) = self.terminal.active_tab {
                        self.sessions.active_tab = Some(active_terminal);
                    }
                }
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.tool_panel_mode
                        || before_page != self.ui.workspace.active_page
                        || before_active_tab != self.sessions.active_tab,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::CloseToolPanel => {
                let before = self.ui.workspace.tool_panel_mode;
                self.ui.workspace.close_tool_panel();
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.tool_panel_mode,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::ToggleRightSidebar => {
                self.ui.workspace.toggle_right_sidebar();
                draft_changed()
            }
            Message::OpenCommandPalette { query } => {
                self.ui.workspace.open_command_palette(query.clone());
                draft_changed()
            }
            Message::UpdateCommandPaletteQuery { query } => {
                self.ui.workspace.command_palette.query = query.clone();
                self.ui.workspace.command_palette.open = true;
                draft_changed()
            }
            Message::CloseCommandPalette => {
                self.ui.workspace.close_command_palette();
                draft_changed()
            }
            Message::NextTheme => {
                self.ui.workspace.next_theme();
                sync_built_in_theme_preference(self)
            }
            Message::SetLanguage { language } => {
                let before = self.ui.workspace.language;
                self.ui.workspace.set_language(*language);
                let changed = before != self.ui.workspace.language;
                self.config.workspace.language = language.preference();
                self.storage.app_config = self.config.clone();
                AppUpdateOutcome {
                    state_changed: changed,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::SetBuiltInTheme { theme } => {
                let before = self.ui.workspace.theme;
                self.ui.workspace.set_built_in_theme(*theme);
                let outcome = sync_built_in_theme_preference(self);
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.theme || outcome.state_changed,
                    ..outcome
                }
            }
            Message::ExportCurrentTheme {
                target_path,
                format,
            } => self
                .core
                .export_current_theme_action(target_path, format.clone()),
            Message::CopyCurrentBuiltInTheme => {
                let outcome = self.core.copy_current_built_in_theme_action();
                if outcome.state_changed {
                    self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
                        &self.config.theme,
                        &self.config.background,
                    );
                }
                outcome
            }
            Message::ImportTheme { source_path } => {
                let outcome = self.core.import_theme_action(source_path);
                if outcome.state_changed {
                    self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
                        &self.config.theme,
                        &self.config.background,
                    );
                }
                outcome
            }
            Message::ApplyThemeProfile { name } => {
                let outcome = self.core.apply_theme_profile_action(name);
                if outcome.state_changed {
                    self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
                        &self.config.theme,
                        &self.config.background,
                    );
                }
                outcome
            }
            Message::RemoveThemeProfile { name } => self.core.remove_theme_profile_action(name),
            Message::BackupStorage { target_path } => self.core.backup_storage_action(target_path),
            Message::ExportStorageSnapshot { target_path } => {
                self.core.export_storage_snapshot_action(target_path)
            }
            Message::ImportStorageSnapshot { source_path } => {
                let outcome = self.core.import_storage_snapshot_action(source_path);
                if outcome.state_changed {
                    self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
                        &self.config.theme,
                        &self.config.background,
                    );
                    let config = self.config.clone();
                    self.ui.apply_workspace_preferences_from_config(&config);
                }
                outcome
            }
            Message::ImportSqliteBackup { source_path } => {
                let outcome = self.core.import_sqlite_backup_action(source_path);
                if outcome.state_changed {
                    self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
                        &self.config.theme,
                        &self.config.background,
                    );
                    let config = self.config.clone();
                    self.ui.apply_workspace_preferences_from_config(&config);
                }
                outcome
            }
            Message::NextBackground => {
                let source_count = self.config.background.normalized().sources.len();
                self.ui.workspace.next_background(source_count);
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

fn sync_built_in_theme_preference(state: &mut AppState) -> AppUpdateOutcome {
    let preference = state.ui.workspace.theme.preference();
    let changed = state.config.workspace.built_in_theme != preference;
    state.config.workspace.built_in_theme = preference;
    state.storage.app_config = state.config.clone();

    AppUpdateOutcome {
        state_changed: changed,
        ..AppUpdateOutcome::default()
    }
}
