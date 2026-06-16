//! 当前桌面 UI 的组合状态和只读视图。
//!
//! 桌面层真正持有两类状态：
//! - `core`: 不依赖具体 GUI 的核心运行态
//! - `ui`: 当前桌面交互草稿
//!
//! 当前桌面主路径直接在这里分流到 `CoreState` 或桌面 `UiState` 草稿。

mod hosts;
mod sftp;
mod sync;
mod visuals;
mod workspace;

use crate::core::CoreState;
use crate::model::{AppUpdateOutcome, CredentialKind, Message, UiState};
use crate::storage::StoragePersistenceError;
use std::cell::Ref;
use std::ops::Deref;
use std::path::Path;

/// 当前桌面适配层共享的组合状态。
#[derive(Debug, Clone)]
pub(crate) struct DesktopAppState {
    pub core: CoreState,
    pub ui: UiState,
}

impl DesktopAppState {
    /// 提交单条桌面消息。
    ///
    /// 纯核心消息直接走 `CoreState`；输入框、弹窗、筛选和页面状态留在桌面
    /// `UiState`，不会再临时拼接历史兼容状态对象。
    pub(crate) fn apply_message(&mut self, message: Message) -> AppUpdateOutcome {
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

    pub(crate) fn view(&self) -> DesktopStateView<'_> {
        DesktopStateView {
            core: &self.core,
            ui: &self.ui,
        }
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
pub(crate) struct DesktopStateView<'a> {
    pub core: &'a CoreState,
    pub ui: &'a UiState,
}

impl Deref for DesktopStateView<'_> {
    type Target = CoreState;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

pub(crate) trait AsDesktopStateView {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_>;
}

impl AsDesktopStateView for DesktopAppState {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_> {
        self.view()
    }
}

impl AsDesktopStateView for &DesktopAppState {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_> {
        (*self).view()
    }
}

impl AsDesktopStateView for DesktopStateView<'_> {
    fn as_desktop_state_view(&self) -> DesktopStateView<'_> {
        *self
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::BackendCommand;
    use crate::model::{
        AgentSource, AuthProfile, CommandHistoryItem, CredentialKind, Host, HostId, KeyAlgorithm,
        RecentConnection, SecretRef, SessionKind, SessionStatus, SnippetScope, WorkspacePage,
    };
    use uuid::Uuid;

    fn desktop_state() -> DesktopAppState {
        let core = CoreState::default();
        let ui = UiState::from_visual(&core.config.theme, &core.config.background);
        DesktopAppState { core, ui }
    }

    fn sample_host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            icon_key: "server".to_owned(),
            tags: vec!["prod".to_owned()],
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                source: AgentSource::Auto,
                key_hint: Some("id_ed25519".to_owned()),
            },
            network: Default::default(),
            proxies: Vec::new(),
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn credential_messages_sync_quick_host_secret_refs_in_desktop_adapter() {
        let mut state = desktop_state();

        for (kind, name, secret_ref) in [
            (
                CredentialKind::PrivateKey,
                "deploy key",
                "secret://keys/deploy",
            ),
            (
                CredentialKind::Certificate,
                "deploy cert",
                "secret://certs/deploy",
            ),
            (
                CredentialKind::Password,
                "deploy password",
                "secret://passwords/deploy",
            ),
        ] {
            let outcome = state.apply_message(Message::CreateCredentialMetadata {
                kind,
                name: name.to_owned(),
                group_id: None,
                secret_ref: secret_ref.to_owned(),
                algorithm: Some(KeyAlgorithm::Ed25519),
            });

            assert!(outcome.changed());
        }

        assert_eq!(
            state.ui.quick_host.auth.private_key_ref,
            SecretRef("secret://keys/deploy".to_owned()).0
        );
        assert_eq!(
            state.ui.quick_host.auth.certificate_ref,
            SecretRef("secret://certs/deploy".to_owned()).0
        );
        assert_eq!(
            state.ui.quick_host.auth.password_secret_ref,
            SecretRef("secret://passwords/deploy".to_owned()).0
        );
    }

    #[test]
    fn desktop_adapter_shell_and_remote_command_messages_activate_terminal_page() {
        let mut shell_state = desktop_state();
        let shell_host = sample_host();
        let shell_host_id = shell_host.id;
        shell_state.core.storage.upsert_host(shell_host);

        let shell_outcome = shell_state.apply_message(Message::OpenShell {
            host_id: shell_host_id,
        });

        assert!(shell_outcome.changed());
        assert_eq!(
            shell_state.ui.workspace.active_page,
            WorkspacePage::Terminal
        );
        assert_eq!(shell_state.core.sessions.tab_count(), 1);
        assert_eq!(shell_state.core.terminal.tab_count(), 1);
        let shell_session_id = shell_state.core.sessions.tabs[0].id;
        let shell_commands = shell_state.core.backend_commands.drain();
        assert!(matches!(
            &shell_commands[0],
            BackendCommand::Connect {
                session_id: command_session_id,
                target,
            } if *command_session_id == shell_session_id
                && target.host_id == shell_host_id
                && target.endpoint() == "example.com:22"
        ));
        assert!(matches!(
            &shell_commands[1],
            BackendCommand::OpenShell {
                session_id: command_session_id,
                pty,
            } if *command_session_id == shell_session_id && pty.term == "xterm-256color"
        ));

        let mut command_state = desktop_state();
        let command_host = sample_host();
        let command_host_id = command_host.id;
        command_state.core.storage.upsert_host(command_host);

        let command_outcome = command_state.apply_message(Message::RunRemoteCommand {
            host_id: command_host_id,
            command: " uptime ".to_owned(),
            request_pty: false,
        });

        assert!(command_outcome.changed());
        assert_eq!(
            command_state.ui.workspace.active_page,
            WorkspacePage::Terminal
        );
        assert_eq!(command_state.core.storage.command_history_count(), 1);
        assert_eq!(
            command_state.core.storage.command_history[0].command,
            "uptime"
        );
        assert!(matches!(
            &command_state.core.sessions.tabs[0].kind,
            SessionKind::RemoteCommand { command, history_id }
                if command == "uptime"
                    && *history_id == Some(command_state.core.storage.command_history[0].id)
        ));
        let command_session_id = command_state.core.sessions.tabs[0].id;
        let command_queue = command_state.core.backend_commands.drain();
        assert!(matches!(
            &command_queue[0],
            BackendCommand::Connect {
                session_id: queued_session_id,
                target,
            } if *queued_session_id == command_session_id
                && target.host_id == command_host_id
        ));
        assert!(matches!(
            &command_queue[1],
            BackendCommand::RunCommand {
                session_id: queued_session_id,
                request,
            } if *queued_session_id == command_session_id
                && request.command == "uptime"
                && request.pty.is_none()
        ));
    }

    #[test]
    fn desktop_adapter_recent_and_reconnect_messages_sync_ui_errors_and_page() {
        let mut recent_state = desktop_state();
        let recent_host = sample_host();
        let recent_host_id = recent_host.id;
        recent_state.core.storage.upsert_host(recent_host);
        recent_state
            .core
            .storage
            .record_recent_connection(RecentConnection {
                host_id: recent_host_id,
                label: "production".to_owned(),
                connected_at_unix_secs: 1,
            });

        let recent_outcome = recent_state.apply_message(Message::OpenRecentConnection {
            host_id: recent_host_id,
        });

        assert!(recent_outcome.changed());
        assert_eq!(
            recent_state.ui.workspace.active_page,
            WorkspacePage::Terminal
        );
        assert_eq!(recent_state.core.sessions.tab_count(), 1);

        let missing_host_id = HostId(Uuid::new_v4());
        let missing_outcome = recent_state.apply_message(Message::OpenRecentConnection {
            host_id: missing_host_id,
        });
        assert!(missing_outcome.error.is_some());
        assert_eq!(
            recent_state.ui.last_error.as_deref(),
            missing_outcome.error.as_deref()
        );

        let mut reconnect_state = desktop_state();
        let reconnect_host = sample_host();
        let reconnect_host_id = reconnect_host.id;
        reconnect_state.core.storage.upsert_host(reconnect_host);
        reconnect_state.apply_message(Message::OpenShell {
            host_id: reconnect_host_id,
        });
        let reconnect_session_id = reconnect_state.core.sessions.tabs[0].id;
        reconnect_state
            .core
            .sessions
            .set_status(reconnect_session_id, SessionStatus::Disconnected);
        reconnect_state.core.backend_commands.drain();

        let reconnect_outcome = reconnect_state.apply_message(Message::ReconnectShell {
            session_id: reconnect_session_id,
        });

        assert!(reconnect_outcome.changed());
        assert_eq!(
            reconnect_state.ui.workspace.active_page,
            WorkspacePage::Terminal
        );
        assert_eq!(
            reconnect_state.core.terminal.active_tab,
            Some(reconnect_session_id)
        );
        assert!(matches!(
            reconnect_state.core.sessions.tabs[0].status,
            SessionStatus::Reconnecting
        ));
    }

    #[test]
    fn desktop_adapter_can_save_host_command_snippet_from_command_draft() {
        let mut state = desktop_state();
        let host = sample_host();
        let host_id = host.id;
        state.core.storage.upsert_host(host);
        state
            .ui
            .set_remote_command(host_id, "systemctl restart {{service}}");

        let outcome = state.apply_message(Message::SaveHostCommandSnippet { host_id });

        assert!(outcome.changed());
        assert_eq!(state.core.storage.snippet_count(), 1);
        assert_eq!(state.core.storage.snippets[0].variables.len(), 1);
        assert_eq!(state.core.storage.snippets[0].variables[0].name, "service");
        assert_eq!(
            state.core.storage.snippets[0].scope,
            SnippetScope::Host(host_id)
        );

        state.ui.set_remote_command(host_id, "   ");
        let rejected = state.apply_message(Message::SaveHostCommandSnippet { host_id });

        assert!(rejected.error.is_some());
        assert_eq!(state.core.storage.snippet_count(), 1);
    }

    #[test]
    fn desktop_adapter_run_command_history_syncs_ui_error() {
        let mut state = desktop_state();
        let history_id = crate::model::CommandHistoryId(Uuid::new_v4());
        state.core.storage.add_command_history(CommandHistoryItem {
            id: history_id,
            host_id: None,
            command: "uptime".to_owned(),
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: 1,
            duration_ms: None,
        });

        let outcome = state.apply_message(Message::RunCommandHistory { history_id });

        assert!(outcome.error.is_some());
        assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    }
}
