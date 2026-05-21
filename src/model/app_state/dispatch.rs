//! 应用消息分发。
//!
//! 这里只负责把 `Message` 路由到具体领域模块，避免根状态文件承担巨大的匹配分发。

#[path = "dispatch/backend.rs"]
mod backend;
#[path = "dispatch/launch.rs"]
mod launch;
#[path = "dispatch/session.rs"]
mod session;
#[path = "dispatch/sftp.rs"]
mod sftp;
#[path = "dispatch/snippets.rs"]
mod snippets;
#[path = "dispatch/storage.rs"]
mod storage;
#[path = "dispatch/ui.rs"]
mod ui;
#[path = "dispatch/visual.rs"]
mod visual;
#[path = "dispatch/workspace.rs"]
mod workspace;

use super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 将 UI 消息应用到根状态。
    pub fn apply(&mut self, message: Message) -> AppUpdateOutcome {
        let mut outcome = self.dispatch_message(message);

        if let Some(error) = &outcome.error {
            outcome.state_changed |= self.ui.set_last_error(error.clone());
        }

        outcome
    }

    fn dispatch_message(&mut self, message: Message) -> AppUpdateOutcome {
        match &message {
            Message::UpdateVisualSettingsDraft { .. }
            | Message::SetVisualBackgroundEnabled { .. }
            | Message::ApplyVisualSettings
            | Message::UpdateHostVisualSettingsDraft { .. }
            | Message::SetHostVisualBackgroundEnabled { .. }
            | Message::ApplyHostVisualSettings { .. }
            | Message::ClearHostVisualSettings { .. } => self.dispatch_visual_message(message),

            Message::SaveWorkspaceSnapshot
            | Message::RestoreWorkspaceSnapshot
            | Message::ClearWorkspaceSnapshot => self.dispatch_workspace_message(message),

            Message::UpdateQuickHostDraft { .. }
            | Message::UpdateQuickHostAuthKind { .. }
            | Message::UpdateQuickHostAuthField { .. }
            | Message::SaveQuickHost
            | Message::DismissUiError
            | Message::SetWorkspacePage { .. }
            | Message::ToggleHostListMode
            | Message::UpdateHostSearchQuery { .. }
            | Message::ResizeHostsPanel { .. }
            | Message::ResizeActivityPanel { .. }
            | Message::ResizeToolPanel { .. }
            | Message::OpenToolPanel { .. }
            | Message::CloseToolPanel
            | Message::ToggleRightSidebar
            | Message::OpenCommandPalette { .. }
            | Message::UpdateCommandPaletteQuery { .. }
            | Message::CloseCommandPalette
            | Message::NextBackground
            | Message::UpdateTerminalInputDraft { .. }
            | Message::AppendTerminalInputDraft { .. }
            | Message::BackspaceTerminalInputDraft { .. }
            | Message::SendTerminalInput { .. }
            | Message::UpdateHostCommandDraft { .. }
            | Message::UpdateHostSftpInitialDirDraft { .. }
            | Message::UpdateSftpActionDraft { .. } => self.dispatch_ui_message(message),

            Message::RemoveCredential { .. }
            | Message::TrustKnownHost { .. }
            | Message::RemoveKnownHost { .. } => self.dispatch_storage_message(message),

            Message::CloseSessionTab { .. } | Message::ActivateTerminalTab { .. } => {
                self.dispatch_session_message(message)
            }

            Message::RefreshSftp { .. }
            | Message::SaveSftpBookmark { .. }
            | Message::OpenSftpBookmark { .. }
            | Message::RemoveSftpBookmark { .. }
            | Message::NavigateSftp { .. }
            | Message::SelectSftpEntry { .. }
            | Message::UploadSftp { .. }
            | Message::DownloadSftp { .. }
            | Message::CancelSftpTransfer { .. }
            | Message::RemoveSftpFile { .. }
            | Message::CreateSftpDir { .. } => self.dispatch_sftp_message(message),

            Message::OpenShell { .. }
            | Message::OpenRecentConnection { .. }
            | Message::OpenSftp { .. }
            | Message::RunRemoteCommand { .. }
            | Message::StartTunnel { .. }
            | Message::StopTunnel { .. } => self.dispatch_launch_message(message),

            Message::SaveHostCommandSnippet { .. }
            | Message::RunSnippet { .. }
            | Message::UpdateSnippetArgument { .. }
            | Message::RemoveSnippet { .. }
            | Message::RunCommandHistory { .. } => self.dispatch_snippet_message(message),

            Message::BackendEventReceived(_) => self.dispatch_backend_message(message),
        }
    }

    fn dismiss_ui_error(&mut self) -> AppUpdateOutcome {
        AppUpdateOutcome {
            state_changed: self.ui.clear_last_error(),
            ..AppUpdateOutcome::default()
        }
    }
}
