//! 应用消息分发。
//!
//! 这里只负责把 `Message` 路由到具体领域模块，避免根状态文件承担巨大的匹配分发。

use super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 将 UI 消息应用到根状态。
    pub fn apply(&mut self, message: Message) -> AppUpdateOutcome {
        let mut outcome = match message {
            Message::UpdateVisualSettingsDraft { field, value } => {
                self.update_visual_settings_draft(field, value)
            }
            Message::SetVisualBackgroundEnabled { enabled } => {
                self.set_visual_background_enabled(enabled)
            }
            Message::ApplyVisualSettings => self.apply_visual_settings(),
            Message::UpdateHostVisualSettingsDraft {
                host_id,
                field,
                value,
            } => self.update_host_visual_settings_draft(host_id, field, value),
            Message::SetHostVisualBackgroundEnabled { host_id, enabled } => {
                self.set_host_visual_background_enabled(host_id, enabled)
            }
            Message::ApplyHostVisualSettings { host_id } => {
                self.apply_host_visual_settings(host_id)
            }
            Message::ClearHostVisualSettings { host_id } => {
                self.clear_host_visual_settings(host_id)
            }
            Message::SaveWorkspaceSnapshot => self.save_workspace_snapshot(),
            Message::RestoreWorkspaceSnapshot => self.restore_workspace_snapshot(),
            Message::ClearWorkspaceSnapshot => self.clear_workspace_snapshot(),
            Message::UpdateQuickHostDraft { field, value } => {
                self.update_quick_host_draft(field, value)
            }
            Message::UpdateQuickHostAuthKind { kind } => self.update_quick_host_auth_kind(kind),
            Message::UpdateQuickHostAuthField { field, value } => {
                self.update_quick_host_auth_field(field, value)
            }
            Message::SaveQuickHost => self.save_quick_host(),
            Message::RemoveCredential { name } => self.remove_credential(&name),
            Message::TrustKnownHost { host, port } => self.trust_known_host(&host, port),
            Message::RemoveKnownHost { host, port } => self.remove_known_host(&host, port),
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
            Message::CloseSessionTab { session_id } => self.close_session_tab(session_id),
            Message::ActivateTerminalTab { session_id } => self.activate_session_tab(session_id),
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
            Message::RefreshSftp { host_id } => self.refresh_sftp(host_id),
            Message::SaveSftpBookmark { host_id } => self.save_sftp_bookmark(host_id),
            Message::OpenSftpBookmark {
                host_id,
                remote_path,
            } => self.open_sftp_bookmark(host_id, remote_path),
            Message::RemoveSftpBookmark {
                host_id,
                remote_path,
            } => self.remove_sftp_bookmark(host_id, remote_path),
            Message::NavigateSftp {
                host_id,
                remote_path,
            } => self.navigate_sftp(host_id, remote_path),
            Message::SelectSftpEntry {
                host_id,
                remote_path,
            } => self.select_sftp_entry(host_id, remote_path),
            Message::UploadSftp { host_id } => self.upload_sftp(host_id),
            Message::DownloadSftp {
                host_id,
                remote_path,
            } => self.download_sftp(host_id, remote_path),
            Message::CancelSftpTransfer { transfer_id } => self.cancel_sftp_transfer(transfer_id),
            Message::RemoveSftpFile {
                host_id,
                remote_path,
            } => self.remove_sftp_file(host_id, remote_path),
            Message::CreateSftpDir { host_id } => self.create_sftp_dir(host_id),
            Message::OpenShell { host_id } => self.open_shell(host_id),
            Message::OpenRecentConnection { host_id } => self.open_recent_connection(host_id),
            Message::OpenSftp {
                host_id,
                initial_dir,
            } => self.open_sftp(host_id, initial_dir),
            Message::RunRemoteCommand {
                host_id,
                command,
                request_pty,
            } => self.run_remote_command(host_id, command, request_pty),
            Message::SaveHostCommandSnippet { host_id } => self.save_host_command_snippet(host_id),
            Message::RunSnippet {
                host_id,
                snippet_id,
            } => self.run_snippet(host_id, snippet_id),
            Message::UpdateSnippetArgument {
                snippet_id,
                name,
                value,
            } => self.update_snippet_argument(snippet_id, name, value),
            Message::RemoveSnippet { snippet_id } => self.remove_snippet(snippet_id),
            Message::RunCommandHistory { history_id } => self.run_command_history(history_id),
            Message::StartTunnel { host_id, rule } => self.start_tunnel(host_id, rule),
            Message::StopTunnel {
                session_id,
                rule_name,
            } => self.stop_tunnel(session_id, rule_name),
            Message::BackendEventReceived(event) => self.apply_backend_event(event),
        };

        if let Some(error) = &outcome.error {
            outcome.state_changed |= self.ui.set_last_error(error.clone());
        }

        outcome
    }

    fn dismiss_ui_error(&mut self) -> AppUpdateOutcome {
        AppUpdateOutcome {
            state_changed: self.ui.clear_last_error(),
            ..AppUpdateOutcome::default()
        }
    }
}
