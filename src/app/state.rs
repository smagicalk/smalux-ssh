//! 当前桌面 UI 的组合状态和只读视图。
//!
//! 桌面层真正持有两类状态：
//! - `core`: 不依赖具体 GUI 的核心运行态
//! - `ui`: 当前桌面交互草稿
//!
//! 旧的 `model::AppState` 仍然保留为测试和兼容入口；当前桌面主路径直接在这里
//! 分流到 `CoreState` 或桌面 `UiState` 草稿。

use crate::config::HostListModePreference;
use crate::core::CoreState;
use crate::model::{
    AppState, AppUpdateOutcome, CredentialKind, Message, QuickHostAuthField, ToolPanelMode,
    UiState, WorkspacePage,
};
use crate::storage::StoragePersistenceError;
use std::cell::Ref;
use std::ops::Deref;
use std::path::Path;

/// 当前桌面适配层共享的组合状态。
#[derive(Debug, Clone)]
pub(super) struct DesktopAppState {
    pub core: CoreState,
    pub ui: UiState,
}

impl DesktopAppState {
    /// 提交单条桌面消息。
    ///
    /// 纯核心消息直接走 `CoreState`；输入框、弹窗、筛选和页面状态留在桌面
    /// `UiState`，不会再临时组装旧 `AppState`。
    pub(super) fn apply_message(&mut self, message: Message) -> AppUpdateOutcome {
        let mut outcome = match message {
            Message::DismissUiError => AppUpdateOutcome {
                state_changed: self.ui.clear_last_error(),
                ..AppUpdateOutcome::default()
            },
            Message::UpdateVisualSettingsDraft { field, value } => {
                self.ui.set_visual_settings_field(field, value);
                draft_changed()
            }
            Message::SetVisualBackgroundEnabled { enabled } => {
                self.ui.set_visual_background_enabled(enabled);
                draft_changed()
            }
            Message::ApplyVisualSettings => self.apply_visual_settings_local(),
            Message::UpdateHostVisualSettingsDraft {
                host_id,
                field,
                value,
            } => self.update_host_visual_settings_draft_local(host_id, field, value),
            Message::SetHostVisualBackgroundEnabled { host_id, enabled } => {
                self.set_host_visual_background_enabled_local(host_id, enabled)
            }
            Message::ApplyHostVisualSettings { host_id } => {
                self.apply_host_visual_settings_local(host_id)
            }
            Message::ClearHostVisualSettings { host_id } => {
                self.clear_host_visual_settings_local(host_id)
            }
            Message::SetWorkspacePage { page } => self.set_workspace_page(page),
            Message::NavigateWorkspacePage { page } => self.navigate_workspace_page(page),
            Message::SaveWorkspaceSnapshot => self.core.save_workspace_snapshot_action(),
            Message::RestoreWorkspaceSnapshot => self.core.restore_workspace_snapshot_action(),
            Message::ClearWorkspaceSnapshot => self.core.clear_workspace_snapshot_action(),
            Message::ToggleHostListMode => self.toggle_host_list_mode(),
            Message::ToggleHostTreeGroup { group_id } => {
                self.ui.workspace.toggle_host_tree_group(group_id);
                draft_changed()
            }
            Message::ToggleCredentialTreeNode { node_id } => {
                self.ui.workspace.toggle_credential_tree_node(node_id);
                draft_changed()
            }
            Message::UpdateHostSearchQuery { query } => {
                self.ui.workspace.set_host_search_query(query);
                draft_changed()
            }
            Message::UpdateCredentialSearchQuery { query } => {
                self.ui.workspace.set_credential_search_query(query);
                draft_changed()
            }
            Message::UpdateSnippetSearchQuery { query } => {
                self.ui.workspace.set_snippet_search_query(query);
                draft_changed()
            }
            Message::UpdateNetworkSearchQuery { query } => {
                self.ui.workspace.set_network_search_query(query);
                draft_changed()
            }
            Message::ToggleSnippetTreeNode { node_id } => {
                self.ui.workspace.toggle_snippet_tree_node(node_id);
                draft_changed()
            }
            Message::UpdateNewSessionSearchQuery { query } => {
                self.ui.workspace.set_new_session_search_query(query);
                draft_changed()
            }
            Message::ResizeHostsPanel { width } => {
                let before = self.ui.workspace.hosts_panel_width;
                self.ui.workspace.set_hosts_panel_width(width);
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.hosts_panel_width,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::ResizeActivityPanel { width } => {
                let before = self.ui.workspace.activity_panel_width;
                self.ui.workspace.set_activity_panel_width(width);
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.activity_panel_width,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::ResizeToolPanel { width } => {
                let before = self.ui.workspace.tool_panel_width;
                self.ui.workspace.set_tool_panel_width(width);
                AppUpdateOutcome {
                    state_changed: before != self.ui.workspace.tool_panel_width,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::OpenToolPanel { mode } => self.open_tool_panel(mode),
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
            Message::NextTheme => self.next_theme_local(),
            Message::SetLanguage { language } => self.set_language_local(language),
            Message::SetBuiltInTheme { theme } => self.set_built_in_theme_local(theme),
            Message::ExportCurrentTheme {
                target_path,
                format,
            } => self.core.export_current_theme_action(&target_path, format),
            Message::CopyCurrentBuiltInTheme => {
                let outcome = self.core.copy_current_built_in_theme_action();
                self.sync_workspace_visuals_from_core(&outcome);
                outcome
            }
            Message::ImportTheme { source_path } => {
                let outcome = self.core.import_theme_action(&source_path);
                self.sync_workspace_visuals_from_core(&outcome);
                outcome
            }
            Message::ApplyThemeProfile { name } => {
                let outcome = self.core.apply_theme_profile_action(&name);
                self.sync_workspace_visuals_from_core(&outcome);
                outcome
            }
            Message::RemoveThemeProfile { name } => self.core.remove_theme_profile_action(&name),
            Message::BackupStorage { target_path } => self.core.backup_storage_action(&target_path),
            Message::ExportStorageSnapshot { target_path } => {
                self.core.export_storage_snapshot_action(&target_path)
            }
            Message::ImportStorageSnapshot { source_path } => {
                let outcome = self.core.import_storage_snapshot_action(&source_path);
                self.sync_workspace_state_from_core_import(&outcome);
                outcome
            }
            Message::ImportSqliteBackup { source_path } => {
                let outcome = self.core.import_sqlite_backup_action(&source_path);
                self.sync_workspace_state_from_core_import(&outcome);
                outcome
            }
            Message::NextBackground => self.next_background_local(),
            Message::OpenCommandPalette { query } => {
                self.ui.workspace.open_command_palette(query);
                draft_changed()
            }
            Message::UpdateCommandPaletteQuery { query } => {
                self.ui.workspace.command_palette.query = query;
                self.ui.workspace.command_palette.open = true;
                draft_changed()
            }
            Message::CloseCommandPalette => {
                self.ui.workspace.close_command_palette();
                draft_changed()
            }
            Message::UpdateQuickHostDraft { field, value } => {
                self.ui.set_quick_host_field(field, value);
                draft_changed()
            }
            Message::SelectQuickHostGroup { group_id } => {
                self.ui.select_quick_host_group(group_id);
                draft_changed()
            }
            Message::UpdateQuickHostAuthKind { kind } => {
                self.ui.set_quick_host_auth_kind(kind);
                draft_changed()
            }
            Message::UpdateQuickHostAuthField { field, value } => {
                self.ui.set_quick_host_auth_field(field, value);
                draft_changed()
            }
            Message::ToggleQuickHostNetworkProxy { proxy_id } => {
                self.toggle_quick_host_network_proxy_local(proxy_id)
            }
            Message::ToggleQuickHostNetworkJumpChain { chain_id } => {
                self.toggle_quick_host_network_jump_chain_local(chain_id)
            }
            Message::ToggleQuickHostNetworkForward { forward_id } => {
                self.toggle_quick_host_network_forward_local(forward_id)
            }
            Message::SaveQuickHost => self.save_quick_host_local(),
            Message::SaveQuickGroup => self.save_quick_group_local(),
            Message::RequestRemoveHost { host_id } => self.request_remove_host_dialog(host_id),
            Message::CancelRemoveHost => self.cancel_remove_host_dialog(),
            Message::ConfirmRemoveHost => self.confirm_remove_host_dialog(),
            Message::RequestRemoveGroup { group_id } => self.request_remove_group_dialog(group_id),
            Message::CancelRemoveGroup => self.cancel_remove_group_dialog(),
            Message::ConfirmRemoveGroup => self.confirm_remove_group_dialog(),
            Message::OpenCreateHostDialog => self.open_create_host_dialog_local(),
            Message::CloseCreateHostDialog => self.close_create_host_dialog_local(),
            Message::OpenEditHostDialog { host_id } => self.open_edit_host_dialog_local(host_id),
            Message::DuplicateHost { host_id } => self.core.duplicate_host_record(host_id),
            Message::OpenCreateHostDialogInGroup { group_id } => {
                self.open_create_host_dialog_in_group_local(group_id)
            }
            Message::OpenCreateGroupParentDialog { parent_id } => {
                self.open_create_group_parent_dialog_local(parent_id)
            }
            Message::SelectCreateGroupParent { parent_id } => {
                self.select_create_group_parent_local(parent_id)
            }
            Message::CloseCreateGroupParentDialog => self.close_create_group_parent_dialog_local(),
            Message::ConfirmCreateGroupParent => self.confirm_create_group_parent_local(),
            Message::OpenCreateGroupDialog { parent_id } => {
                self.open_create_group_dialog_local(parent_id)
            }
            Message::UpdateQuickGroupName { name } => {
                self.ui.quick_group.name = name;
                draft_changed()
            }
            Message::SelectQuickGroupParent { parent_id } => {
                self.select_quick_group_parent_local(parent_id)
            }
            Message::CloseCreateGroupDialog => self.close_create_group_dialog_local(),
            Message::OpenLocalTerminal => {
                let outcome = self.core.open_local_terminal_action();
                self.activate_terminal_page_for(&outcome);
                if outcome.changed() {
                    self.ui.workspace.set_hosts_panel_collapsed(false);
                }
                outcome
            }
            Message::UpdateTerminalInputDraft { session_id, input } => {
                self.ui.set_terminal_input(session_id, input);
                draft_changed()
            }
            Message::AppendTerminalInputDraft { session_id, text } => {
                let filtered = printable_terminal_input(&text);
                if filtered.is_empty() {
                    AppUpdateOutcome::default()
                } else {
                    self.ui.append_terminal_input(session_id, filtered);
                    draft_changed()
                }
            }
            Message::BackspaceTerminalInputDraft { session_id } => {
                let before = self.ui.terminal_input_for(session_id).to_owned();
                self.ui.backspace_terminal_input(session_id);
                AppUpdateOutcome {
                    state_changed: before != self.ui.terminal_input_for(session_id),
                    ..AppUpdateOutcome::default()
                }
            }
            Message::SendTerminalInput { session_id } => {
                let input = self.ui.terminal_input_for(session_id).to_owned();
                let outcome = self.core.send_terminal_input_action(session_id, input);
                if outcome.error.is_none() && outcome.state_changed {
                    self.ui.clear_terminal_input(session_id);
                }
                outcome
            }
            Message::UpdateHostCommandDraft { host_id, command } => {
                self.ui.set_remote_command(host_id, command);
                draft_changed()
            }
            Message::UpdateHostSftpInitialDirDraft {
                host_id,
                initial_dir,
            } => {
                self.ui.set_sftp_initial_dir(host_id, initial_dir);
                draft_changed()
            }
            Message::UpdateSftpActionDraft {
                host_id,
                field,
                value,
            } => {
                self.ui.set_sftp_action_field(host_id, field, value);
                draft_changed()
            }
            Message::BackendEventReceived(_)
            | Message::CloseSessionTab { .. }
            | Message::ActivateTerminalTab { .. }
            | Message::RefreshSftp { .. }
            | Message::SaveSftpBookmark { .. }
            | Message::OpenSftpBookmark { .. }
            | Message::RemoveSftpBookmark { .. }
            | Message::NavigateSftp { .. }
            | Message::SelectSftpEntry { .. }
            | Message::CancelSftpTransfer { .. }
            | Message::RemoveSftpFile { .. }
            | Message::SaveProxyAsset { .. }
            | Message::SaveJumpChainAsset { .. }
            | Message::SaveForwardAsset { .. }
            | Message::RemoveProxyAsset { .. }
            | Message::RemoveJumpChainAsset { .. }
            | Message::RemoveForwardAsset { .. }
            | Message::TrustKnownHost { .. }
            | Message::RemoveKnownHost { .. }
            | Message::CreateCredentialGroup { .. }
            | Message::RenameCredentialGroup { .. }
            | Message::RemoveCredentialGroup { .. }
            | Message::UpdateCredentialMetadata { .. }
            | Message::UpdateCredentialSecret { .. }
            | Message::ExportCredentialSecret { .. }
            | Message::DuplicateCredential { .. }
            | Message::RemoveCredential { .. }
            | Message::MoveCredential { .. }
            | Message::MoveCredentialGroup { .. } => self.core.apply_core_message(message),
            Message::UploadSftp { host_id } => self.upload_sftp_local(host_id),
            Message::DownloadSftp {
                host_id,
                remote_path,
            } => self.download_sftp_local(host_id, remote_path),
            Message::CreateSftpDir { host_id } => self.create_sftp_dir_local(host_id),
            Message::UpdateSnippetArgument { .. }
            | Message::CreateSnippet { .. }
            | Message::UpdateSnippet { .. }
            | Message::CreateSnippetTarget { .. }
            | Message::UpdateSnippetTarget { .. }
            | Message::SyncSnippetTargetImplementationTargets { .. }
            | Message::RemoveSnippetTarget { .. }
            | Message::SplitSnippetTargetImplementation { .. }
            | Message::CreateSnippetGroup { .. }
            | Message::RenameSnippetGroup { .. }
            | Message::RemoveSnippetGroup { .. }
            | Message::RemoveSnippetGroupRecursive { .. }
            | Message::MoveSnippetGroup { .. }
            | Message::MoveSnippet { .. }
            | Message::RemoveSnippet { .. } => self.core.apply_core_message(message),
            Message::SaveHostCommandSnippet { host_id } => {
                let command = self.ui.remote_command_for(host_id).to_owned();
                self.core.save_host_command_snippet_action(host_id, command)
            }
            Message::OpenShell { host_id } => {
                let outcome = self.core.open_shell_action(host_id);
                self.activate_terminal_page_for(&outcome);
                outcome
            }
            Message::OpenRecentConnection { host_id } => {
                let outcome = self.core.open_recent_connection_action(host_id);
                self.activate_terminal_page_for(&outcome);
                outcome
            }
            Message::ReconnectShell { session_id } => {
                let outcome = self.core.reconnect_shell_action(session_id);
                self.activate_terminal_page_for(&outcome);
                outcome
            }
            Message::OpenSftp {
                host_id,
                initial_dir,
            } => {
                let outcome = self.core.open_sftp_action(host_id, initial_dir);
                self.activate_sftp_page_for(&outcome);
                outcome
            }
            Message::RunRemoteCommand {
                host_id,
                command,
                request_pty,
            } => {
                let outcome = self.core.run_remote_command(host_id, command, request_pty);
                self.activate_terminal_page_for(&outcome);
                outcome
            }
            Message::StartTunnel { host_id, rule } => {
                let outcome = self.core.start_tunnel_action(host_id, rule);
                self.activate_tunnel_page_for(&outcome);
                outcome
            }
            Message::StopTunnel {
                session_id,
                rule_name,
            } => self.core.stop_tunnel_action(session_id, rule_name),
            Message::RunSnippet {
                host_id,
                snippet_id,
            } => {
                let outcome = self.core.run_snippet_action(host_id, snippet_id);
                self.activate_terminal_page_for(&outcome);
                outcome
            }
            Message::RunSnippetWithArguments {
                host_id,
                snippet_id,
                arguments,
            } => {
                let outcome = self
                    .core
                    .run_snippet_with_arguments_action(host_id, snippet_id, arguments);
                self.activate_terminal_page_for(&outcome);
                outcome
            }
            Message::RunSnippetTargetWithArguments {
                host_id,
                snippet_id,
                target_id,
                arguments,
            } => {
                let outcome = self.core.run_snippet_target_with_arguments_action(
                    host_id, snippet_id, target_id, arguments,
                );
                self.activate_terminal_page_for(&outcome);
                outcome
            }
            Message::RunSnippetOnActiveHost { snippet_id } => {
                if let Some(host_id) = self.active_remote_host_id() {
                    let outcome = self.core.run_snippet_action(host_id, snippet_id);
                    self.activate_terminal_page_for(&outcome);
                    outcome
                } else {
                    AppUpdateOutcome {
                        error: Some("请先打开或选中一个远程主机终端".to_owned()),
                        ..AppUpdateOutcome::default()
                    }
                }
            }
            Message::RunSnippetTargetOnActiveHost {
                snippet_id,
                target_id,
            } => {
                if let Some(host_id) = self.active_remote_host_id() {
                    let outcome = self
                        .core
                        .run_snippet_target_action(host_id, snippet_id, target_id);
                    self.activate_terminal_page_for(&outcome);
                    outcome
                } else {
                    AppUpdateOutcome {
                        error: Some("请先打开或选中一个远程主机终端".to_owned()),
                        ..AppUpdateOutcome::default()
                    }
                }
            }
            Message::RunCommandHistory { history_id } => {
                let outcome = self.core.run_command_history(history_id);
                self.activate_terminal_page_for(&outcome);
                outcome
            }
            Message::CreateCredentialMetadata {
                kind,
                name,
                group_id,
                secret_ref,
                algorithm,
            } => {
                let outcome = self.core.create_credential_metadata_action(
                    kind.clone(),
                    name.clone(),
                    group_id,
                    secret_ref,
                    algorithm,
                );
                self.sync_quick_host_credential_ref(kind, &name, &outcome);
                outcome
            }
            Message::GeneratePrivateKeyCredential {
                name,
                group_id,
                algorithm,
            } => {
                let outcome = self.core.generate_private_key_credential_action(
                    name.clone(),
                    group_id,
                    algorithm,
                );
                self.sync_quick_host_credential_ref(CredentialKind::PrivateKey, &name, &outcome);
                outcome
            }
            Message::SavePasswordCredential {
                name,
                group_id,
                password,
            } => {
                let outcome =
                    self.core
                        .save_password_credential_action(name.clone(), group_id, password);
                self.sync_quick_host_credential_ref(CredentialKind::Password, &name, &outcome);
                outcome
            }
            Message::ImportPrivateKeyCredential {
                name,
                group_id,
                source_path,
                algorithm,
            } => {
                let outcome = self.core.import_private_key_credential_action(
                    name.clone(),
                    group_id,
                    source_path,
                    algorithm,
                );
                self.sync_quick_host_credential_ref(CredentialKind::PrivateKey, &name, &outcome);
                outcome
            }
            Message::ImportPrivateKeyTextCredential {
                name,
                group_id,
                private_key_text,
                algorithm,
            } => {
                let outcome = self.core.import_private_key_text_credential_action(
                    name.clone(),
                    group_id,
                    private_key_text,
                    algorithm,
                );
                self.sync_quick_host_credential_ref(CredentialKind::PrivateKey, &name, &outcome);
                outcome
            }
            Message::ImportCertificateCredential {
                name,
                group_id,
                source_path,
                algorithm,
            } => {
                let outcome = self.core.import_certificate_credential_action(
                    name.clone(),
                    group_id,
                    source_path,
                    algorithm,
                );
                self.sync_quick_host_credential_ref(CredentialKind::Certificate, &name, &outcome);
                outcome
            }
            Message::ImportCertificateTextCredential {
                name,
                group_id,
                certificate_text,
                algorithm,
            } => {
                let outcome = self.core.import_certificate_text_credential_action(
                    name.clone(),
                    group_id,
                    certificate_text,
                    algorithm,
                );
                self.sync_quick_host_credential_ref(CredentialKind::Certificate, &name, &outcome);
                outcome
            }
            Message::GenerateCertificateCredential {
                name,
                group_id,
                ca_private_key_ref,
                subject_private_key_ref,
                cert_type,
                principals,
                valid_days,
                key_id,
                serial,
            } => {
                let outcome = self.core.generate_certificate_credential_action(
                    name.clone(),
                    group_id,
                    ca_private_key_ref,
                    subject_private_key_ref,
                    cert_type,
                    principals,
                    valid_days,
                    key_id,
                    serial,
                );
                self.sync_quick_host_credential_ref(CredentialKind::Certificate, &name, &outcome);
                outcome
            }
        };

        if let Some(error) = &outcome.error {
            outcome.state_changed |= self.ui.set_last_error(error.clone());
        }

        outcome
    }

    /// 按顺序提交一组桌面消息，并合并状态变更结果。
    pub(super) fn apply_messages(
        &mut self,
        messages: impl IntoIterator<Item = Message>,
    ) -> AppUpdateOutcome {
        let mut merged = AppUpdateOutcome::default();
        for message in messages {
            merge_outcome(&mut merged, self.apply_message(message));
        }
        merged
    }

    /// 提交消息并在持久化数据变化后落盘。
    pub(super) fn apply_messages_with_persistence(
        &mut self,
        messages: impl IntoIterator<Item = Message>,
    ) -> (AppUpdateOutcome, Option<StoragePersistenceError>) {
        let storage_before = self.core.storage.clone();
        let outcome = self.apply_messages(messages);
        let persist_error = if self.core.storage != storage_before {
            self.core.persist_storage().err()
        } else {
            None
        };

        (outcome, persist_error)
    }

    pub(super) fn view(&self) -> DesktopStateView<'_> {
        DesktopStateView {
            core: &self.core,
            ui: &self.ui,
        }
    }

    fn sync_quick_host_credential_ref(
        &mut self,
        kind: CredentialKind,
        credential_name: &str,
        outcome: &AppUpdateOutcome,
    ) {
        if !outcome.changed() {
            return;
        }

        let auth_field = match kind {
            CredentialKind::Password => QuickHostAuthField::PasswordSecretRef,
            CredentialKind::PrivateKey => QuickHostAuthField::PrivateKeyRef,
            CredentialKind::Certificate => QuickHostAuthField::CertificateRef,
            CredentialKind::Agent => return,
        };

        let Some(secret_ref) = self
            .core
            .storage
            .credentials
            .iter()
            .find(|credential| credential.name == credential_name && credential.kind == kind)
            .and_then(|credential| credential.secret.as_ref())
        else {
            return;
        };

        self.ui
            .set_quick_host_auth_field(auth_field, secret_ref.0.as_str());
    }

    fn sync_workspace_visuals_from_core(&mut self, outcome: &AppUpdateOutcome) {
        if !outcome.state_changed {
            return;
        }

        self.ui.visual_settings = crate::model::VisualSettingsDraft::from_profiles(
            &self.core.config.theme,
            &self.core.config.background,
        );
    }

    fn sync_workspace_state_from_core_import(&mut self, outcome: &AppUpdateOutcome) {
        if !outcome.state_changed {
            return;
        }

        self.sync_workspace_visuals_from_core(outcome);
        self.ui.workspace.host_list_mode = match self.core.config.workspace.host_list_mode {
            HostListModePreference::Tree => crate::model::HostListMode::Tree,
            HostListModePreference::Card => crate::model::HostListMode::Card,
        };
        self.ui.workspace.language =
            crate::model::LanguageMode::from_preference(self.core.config.workspace.language);
        self.ui.workspace.theme =
            crate::model::BuiltInTheme::from_preference(self.core.config.workspace.built_in_theme);
    }

    fn apply_visual_settings_local(&mut self) -> AppUpdateOutcome {
        let draft = self.ui.visual_settings.clone();
        let theme = match draft.build_theme_profile(&self.core.config.theme) {
            Ok(theme) => theme,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };
        let background = match draft.build_background_profile(&self.core.config.background) {
            Ok(background) => background,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };

        let outcome = self.core.apply_visual_profiles_action(theme, background);
        self.sync_workspace_visuals_from_core(&outcome);
        outcome
    }

    fn update_host_visual_settings_draft_local(
        &mut self,
        host_id: crate::model::HostId,
        field: crate::model::VisualSettingsDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        let Some((theme, background)) = self.host_visual_fallbacks(host_id) else {
            return missing_host(host_id);
        };

        self.ui
            .set_host_visual_settings_field(host_id, field, value, &theme, &background);
        draft_changed()
    }

    fn set_host_visual_background_enabled_local(
        &mut self,
        host_id: crate::model::HostId,
        enabled: bool,
    ) -> AppUpdateOutcome {
        let Some((theme, background)) = self.host_visual_fallbacks(host_id) else {
            return missing_host(host_id);
        };

        self.ui
            .set_host_visual_background_enabled(host_id, enabled, &theme, &background);
        draft_changed()
    }

    fn apply_host_visual_settings_local(
        &mut self,
        host_id: crate::model::HostId,
    ) -> AppUpdateOutcome {
        let Some((fallback_theme, fallback_background)) = self.host_visual_fallbacks(host_id)
        else {
            return missing_host(host_id);
        };
        let draft = self
            .ui
            .host_visual_settings_for(host_id)
            .cloned()
            .unwrap_or_else(|| {
                crate::model::VisualSettingsDraft::from_profiles(
                    &fallback_theme,
                    &fallback_background,
                )
            });
        let theme = match draft.build_theme_profile(&fallback_theme) {
            Ok(theme) => theme,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };
        let background = match draft.build_background_profile(&fallback_background) {
            Ok(background) => background,
            Err(error) => return invalid_visual_settings(error.to_string()),
        };

        let outcome = self
            .core
            .apply_host_visual_profiles_action(host_id, theme, background);
        self.ui.clear_host_visual_settings_draft(host_id);
        outcome
    }

    fn clear_host_visual_settings_local(
        &mut self,
        host_id: crate::model::HostId,
    ) -> AppUpdateOutcome {
        let outcome = self.core.clear_host_visual_profiles_action(host_id);
        self.ui.clear_host_visual_settings_draft(host_id);
        outcome
    }

    fn host_visual_fallbacks(
        &self,
        host_id: crate::model::HostId,
    ) -> Option<(crate::model::ThemeProfile, crate::model::BackgroundProfile)> {
        self.core
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .map(|host| {
                (
                    host.theme_override
                        .clone()
                        .unwrap_or_else(|| self.core.config.theme.clone()),
                    host.background_override
                        .clone()
                        .unwrap_or_else(|| self.core.config.background.clone()),
                )
            })
    }

    fn upload_sftp_local(&mut self, host_id: crate::model::HostId) -> AppUpdateOutcome {
        let local_path = self.ui.sftp_local_path_for(host_id).trim().to_owned();
        if local_path.is_empty() {
            return AppUpdateOutcome {
                error: Some("SFTP 本地路径不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let remote_name = self.ui.sftp_remote_name_for(host_id).trim();
        let remote_name = if remote_name.is_empty() {
            match basename_local_path(&local_path) {
                Some(name) => name,
                None => {
                    return AppUpdateOutcome {
                        error: Some("无法从本地路径推断远程文件名".to_owned()),
                        ..AppUpdateOutcome::default()
                    };
                }
            }
        } else {
            remote_name.to_owned()
        };

        self.core
            .upload_sftp_with_paths_action(host_id, local_path, remote_name)
    }

    fn download_sftp_local(
        &mut self,
        host_id: crate::model::HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = remote_path.trim().to_owned();
        if remote_path.is_empty() || remote_path == "/" {
            return AppUpdateOutcome {
                error: Some("SFTP 下载路径不能为空或根目录".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let local_path = self.ui.sftp_local_path_for(host_id).trim().to_owned();
        let local_path = if local_path.is_empty() {
            match basename_local_path(&remote_path) {
                Some(name) => name,
                None => {
                    return AppUpdateOutcome {
                        error: Some("SFTP 本地路径不能为空".to_owned()),
                        ..AppUpdateOutcome::default()
                    };
                }
            }
        } else {
            local_path
        };

        self.core
            .download_sftp_to_path_action(host_id, remote_path, local_path)
    }

    fn create_sftp_dir_local(&mut self, host_id: crate::model::HostId) -> AppUpdateOutcome {
        let new_dir_name = self.ui.sftp_new_dir_name_for(host_id).trim().to_owned();
        self.core
            .create_sftp_dir_named_action(host_id, new_dir_name)
    }

    fn activate_terminal_page_for(&mut self, outcome: &AppUpdateOutcome) {
        if outcome.changed() {
            self.ui.workspace.active_page = WorkspacePage::Terminal;
        }
    }

    fn activate_sftp_page_for(&mut self, outcome: &AppUpdateOutcome) {
        if outcome.changed() {
            self.ui.workspace.active_page = WorkspacePage::Sftp;
        }
    }

    fn activate_tunnel_page_for(&mut self, outcome: &AppUpdateOutcome) {
        if outcome.changed() {
            self.ui.workspace.active_page = WorkspacePage::Tunnels;
        }
    }

    fn set_workspace_page(&mut self, page: WorkspacePage) -> AppUpdateOutcome {
        self.ui.workspace.active_page = page;
        self.ui.workspace.set_hosts_panel_collapsed(false);
        draft_changed()
    }

    fn navigate_workspace_page(&mut self, page: WorkspacePage) -> AppUpdateOutcome {
        if self.ui.workspace.active_page == page {
            let collapsed = !self.ui.workspace.hosts_panel_collapsed;
            self.ui.workspace.set_hosts_panel_collapsed(collapsed);
        } else {
            self.ui.workspace.active_page = page;
            self.ui.workspace.set_hosts_panel_collapsed(false);
        }
        draft_changed()
    }

    fn toggle_host_list_mode(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_host_list_mode();
        let preference = match self.ui.workspace.host_list_mode {
            crate::model::HostListMode::Tree => HostListModePreference::Tree,
            crate::model::HostListMode::Card => HostListModePreference::Card,
        };
        let changed = self.core.config.workspace.host_list_mode != preference;
        self.core.config.workspace.host_list_mode = preference;
        self.core.storage.app_config = self.core.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    fn open_tool_panel(&mut self, mode: ToolPanelMode) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_mode;
        let before_page = self.ui.workspace.active_page;
        let before_active_tab = self.core.sessions.active_tab;
        self.ui.workspace.open_tool_panel(mode);
        if matches!(mode, ToolPanelMode::Sftp) {
            self.ui.workspace.active_page = WorkspacePage::Terminal;
            if let Some(active_terminal) = self.core.terminal.active_tab {
                self.core.sessions.active_tab = Some(active_terminal);
            }
        }
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_mode
                || before_page != self.ui.workspace.active_page
                || before_active_tab != self.core.sessions.active_tab,
            ..AppUpdateOutcome::default()
        }
    }

    fn active_remote_host_id(&self) -> Option<crate::model::HostId> {
        let active_tab = self.core.sessions.active_tab?;
        self.core
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == active_tab)
            .and_then(|tab| tab.host_id)
    }

    fn next_theme_local(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.next_theme();
        self.sync_built_in_theme_preference()
    }

    fn set_language_local(&mut self, language: crate::model::LanguageMode) -> AppUpdateOutcome {
        let before = self.ui.workspace.language;
        self.ui.workspace.set_language(language);
        let changed = before != self.ui.workspace.language;
        self.core.config.workspace.language = language.preference();
        self.core.storage.app_config = self.core.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    fn set_built_in_theme_local(&mut self, theme: crate::model::BuiltInTheme) -> AppUpdateOutcome {
        let before = self.ui.workspace.theme;
        self.ui.workspace.set_built_in_theme(theme);
        let outcome = self.sync_built_in_theme_preference();
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.theme || outcome.state_changed,
            ..outcome
        }
    }

    fn sync_built_in_theme_preference(&mut self) -> AppUpdateOutcome {
        let preference = self.ui.workspace.theme.preference();
        let changed = self.core.config.workspace.built_in_theme != preference;
        self.core.config.workspace.built_in_theme = preference;
        self.core.storage.app_config = self.core.config.clone();

        AppUpdateOutcome {
            state_changed: changed,
            ..AppUpdateOutcome::default()
        }
    }

    fn next_background_local(&mut self) -> AppUpdateOutcome {
        let source_count = self.core.config.background.normalized().sources.len();
        self.ui.workspace.next_background(source_count);
        draft_changed()
    }

    fn request_remove_host_dialog(&mut self, host_id: crate::model::HostId) -> AppUpdateOutcome {
        if self
            .core
            .storage
            .hosts
            .iter()
            .any(|host| host.id == host_id)
        {
            self.ui.workspace.pending_delete_host_id = Some(host_id);
            return draft_changed();
        }
        AppUpdateOutcome {
            error: Some("主机不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }

    fn toggle_quick_host_network_proxy_local(
        &mut self,
        proxy_id: crate::model::ProxyId,
    ) -> AppUpdateOutcome {
        if !self
            .core
            .storage
            .proxy_assets
            .iter()
            .any(|asset| asset.id == proxy_id)
        {
            return AppUpdateOutcome {
                error: Some("代理资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_proxy(proxy_id);
        draft_changed()
    }

    fn toggle_quick_host_network_jump_chain_local(
        &mut self,
        chain_id: crate::model::JumpChainId,
    ) -> AppUpdateOutcome {
        if !self
            .core
            .storage
            .jump_chain_assets
            .iter()
            .any(|asset| asset.id == chain_id)
        {
            return AppUpdateOutcome {
                error: Some("跳板资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_jump_chain(chain_id);
        draft_changed()
    }

    fn toggle_quick_host_network_forward_local(
        &mut self,
        forward_id: crate::model::ForwardId,
    ) -> AppUpdateOutcome {
        if !self
            .core
            .storage
            .forward_assets
            .iter()
            .any(|asset| asset.id == forward_id)
        {
            return AppUpdateOutcome {
                error: Some("转发资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_forward(forward_id);
        draft_changed()
    }

    fn save_quick_host_local(&mut self) -> AppUpdateOutcome {
        let editing_host_id = self.ui.quick_host.editing_host_id;
        let existing_host = editing_host_id.and_then(|host_id| {
            self.core
                .storage
                .hosts
                .iter()
                .find(|host| host.id == host_id)
                .cloned()
        });

        if editing_host_id.is_some() && existing_host.is_none() {
            return AppUpdateOutcome {
                error: Some("主机不存在，无法保存编辑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let host_id = editing_host_id.unwrap_or_else(|| crate::model::HostId(uuid::Uuid::new_v4()));
        let host = match self
            .ui
            .quick_host
            .build_host_with_existing(host_id, existing_host.as_ref())
        {
            Ok(host) => host,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("主机表单无效：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        let outcome = self.core.save_host_record(host, editing_host_id);
        if outcome.error.is_some() {
            return outcome;
        }
        self.ui.reset_quick_host();
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    fn save_quick_group_local(&mut self) -> AppUpdateOutcome {
        let name = self.ui.quick_group.name.trim().to_owned();
        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("分组名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let parent_id = self.ui.quick_group.parent_id;
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法保存".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let outcome = self.core.save_group_record(crate::model::HostGroup {
            id: crate::model::GroupId(uuid::Uuid::new_v4()),
            name,
            parent_id,
        });
        if outcome.error.is_some() {
            return outcome;
        }
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.quick_group = crate::model::QuickGroupDraft::default();
        draft_changed()
    }

    fn cancel_remove_host_dialog(&mut self) -> AppUpdateOutcome {
        let had_pending = self.ui.workspace.pending_delete_host_id.take().is_some();
        AppUpdateOutcome {
            state_changed: had_pending,
            ..AppUpdateOutcome::default()
        }
    }

    fn confirm_remove_host_dialog(&mut self) -> AppUpdateOutcome {
        let Some(host_id) = self.ui.workspace.pending_delete_host_id.take() else {
            return AppUpdateOutcome {
                error: Some("没有待删除的主机".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.core.remove_host_record_action(host_id)
    }

    fn request_remove_group_dialog(&mut self, group_id: crate::model::GroupId) -> AppUpdateOutcome {
        if self
            .core
            .storage
            .groups
            .iter()
            .any(|group| group.id == group_id)
        {
            self.ui.workspace.pending_delete_group_id = Some(group_id);
            return draft_changed();
        }
        AppUpdateOutcome {
            error: Some("分组不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }

    fn cancel_remove_group_dialog(&mut self) -> AppUpdateOutcome {
        let had_pending = self.ui.workspace.pending_delete_group_id.take().is_some();
        AppUpdateOutcome {
            state_changed: had_pending,
            ..AppUpdateOutcome::default()
        }
    }

    fn confirm_remove_group_dialog(&mut self) -> AppUpdateOutcome {
        let Some(group_id) = self.ui.workspace.pending_delete_group_id.take() else {
            return AppUpdateOutcome {
                error: Some("没有待删除的分组".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.core.remove_group_record_recursive_action(group_id)
    }

    fn open_create_host_dialog_local(&mut self) -> AppUpdateOutcome {
        self.ui.reset_quick_host();
        self.ui.workspace.create_host_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        draft_changed()
    }

    fn close_create_host_dialog_local(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    fn open_edit_host_dialog_local(&mut self, host_id: crate::model::HostId) -> AppUpdateOutcome {
        let Some(host) = self
            .core
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some("主机不存在，无法编辑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.ui.edit_quick_host(&host);
        self.ui.workspace.create_host_dialog_open = true;
        draft_changed()
    }

    fn open_create_host_dialog_in_group_local(
        &mut self,
        group_id: Option<crate::model::GroupId>,
    ) -> AppUpdateOutcome {
        if group_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id)) {
            return AppUpdateOutcome {
                error: Some("分组不存在，无法创建主机".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.reset_quick_host();
        self.ui.quick_host.group_id = group_id;
        self.ui.workspace.create_host_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        draft_changed()
    }

    fn open_create_group_parent_dialog_local(
        &mut self,
        parent_id: Option<crate::model::GroupId>,
    ) -> AppUpdateOutcome {
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.workspace.pending_create_group_parent_id = parent_id;
        self.ui.workspace.create_group_parent_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    fn select_create_group_parent_local(
        &mut self,
        parent_id: Option<crate::model::GroupId>,
    ) -> AppUpdateOutcome {
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        self.ui.workspace.pending_create_group_parent_id = parent_id;
        draft_changed()
    }

    fn close_create_group_parent_dialog_local(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        draft_changed()
    }

    fn confirm_create_group_parent_local(&mut self) -> AppUpdateOutcome {
        let parent_id = self.ui.workspace.pending_create_group_parent_id;
        self.open_create_group_dialog_local(parent_id)
    }

    fn open_create_group_dialog_local(
        &mut self,
        parent_id: Option<crate::model::GroupId>,
    ) -> AppUpdateOutcome {
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法创建子分组".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.quick_group = crate::model::QuickGroupDraft::with_parent(parent_id);
        self.ui.workspace.create_group_dialog_open = true;
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    fn select_quick_group_parent_local(
        &mut self,
        parent_id: Option<crate::model::GroupId>,
    ) -> AppUpdateOutcome {
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        self.ui.quick_group.parent_id = parent_id;
        draft_changed()
    }

    fn close_create_group_dialog_local(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.quick_group = crate::model::QuickGroupDraft::default();
        draft_changed()
    }
}

fn merge_outcome(merged: &mut AppUpdateOutcome, outcome: AppUpdateOutcome) {
    merged.state_changed |= outcome.state_changed;
    merged.queued_backend_commands += outcome.queued_backend_commands;
    merged.executed_backend_commands += outcome.executed_backend_commands;
    merged.applied_backend_events += outcome.applied_backend_events;
    if merged.worker_command.is_none() {
        merged.worker_command = outcome.worker_command;
    }
    if merged.error.is_none() {
        merged.error = outcome.error;
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

fn invalid_visual_settings(error: String) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("视觉配置无效：{error}")),
        ..AppUpdateOutcome::default()
    }
}

fn missing_host(host_id: crate::model::HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn basename_local_path(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(ToOwned::to_owned)
}

#[derive(Clone, Copy)]
pub(super) struct DesktopStateView<'a> {
    pub core: &'a CoreState,
    pub ui: &'a UiState,
}

impl Deref for DesktopStateView<'_> {
    type Target = CoreState;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

pub(super) trait AsDesktopStateView {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_>;
}

impl AsDesktopStateView for DesktopAppState {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_> {
        self.view()
    }
}

impl AsDesktopStateView for DesktopStateView<'_> {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_> {
        *self
    }
}

impl AsDesktopStateView for AppState {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_> {
        DesktopStateView {
            core: &self.core,
            ui: &self.ui,
        }
    }
}

impl<T> AsDesktopStateView for Ref<'_, T>
where
    T: AsDesktopStateView,
{
    fn as_desktop_state_view(&self) -> DesktopStateView<'_> {
        (**self).as_desktop_state_view()
    }
}

impl AsDesktopStateView for &AppState {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_> {
        (*self).as_desktop_state_view()
    }
}
