use super::super::Message;

#[cfg(test)]
#[path = "target_tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MessageDispatchTarget {
    Visual,
    Workspace,
    Ui,
    Storage,
    Session,
    Sftp,
    Launch,
    Snippet,
    Backend,
}

impl MessageDispatchTarget {
    pub(super) fn for_message(message: &Message) -> Self {
        match message {
            Message::UpdateVisualSettingsDraft { .. }
            | Message::SetVisualBackgroundEnabled { .. }
            | Message::ApplyVisualSettings
            | Message::UpdateHostVisualSettingsDraft { .. }
            | Message::SetHostVisualBackgroundEnabled { .. }
            | Message::ApplyHostVisualSettings { .. }
            | Message::ClearHostVisualSettings { .. } => Self::Visual,

            Message::SaveWorkspaceSnapshot
            | Message::RestoreWorkspaceSnapshot
            | Message::ClearWorkspaceSnapshot => Self::Workspace,

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
            | Message::UpdateSftpActionDraft { .. } => Self::Ui,

            Message::RemoveCredential { .. }
            | Message::TrustKnownHost { .. }
            | Message::RemoveKnownHost { .. } => Self::Storage,

            Message::CloseSessionTab { .. } | Message::ActivateTerminalTab { .. } => Self::Session,

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
            | Message::CreateSftpDir { .. } => Self::Sftp,

            Message::OpenShell { .. }
            | Message::OpenRecentConnection { .. }
            | Message::ReconnectShell { .. }
            | Message::OpenSftp { .. }
            | Message::RunRemoteCommand { .. }
            | Message::StartTunnel { .. }
            | Message::StopTunnel { .. } => Self::Launch,

            Message::SaveHostCommandSnippet { .. }
            | Message::RunSnippet { .. }
            | Message::UpdateSnippetArgument { .. }
            | Message::RemoveSnippet { .. }
            | Message::RunCommandHistory { .. } => Self::Snippet,

            Message::BackendEventReceived(_) => Self::Backend,
        }
    }
}
