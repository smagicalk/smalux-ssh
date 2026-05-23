use super::super::Message;

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

#[cfg(test)]
mod tests {
    use crate::backend::BackendEvent;
    use crate::model::{
        HostId, QuickHostDraftField, SessionId, VisualSettingsDraftField, WorkspacePage,
    };

    use super::*;

    #[test]
    fn classifies_representative_messages_by_dispatch_target() {
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::UpdateVisualSettingsDraft {
                field: VisualSettingsDraftField::FontSize,
                value: "16".to_owned(),
            }),
            MessageDispatchTarget::Visual
        );
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::SaveWorkspaceSnapshot),
            MessageDispatchTarget::Workspace
        );
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::UpdateQuickHostDraft {
                field: QuickHostDraftField::Address,
                value: "example.com".to_owned(),
            }),
            MessageDispatchTarget::Ui
        );
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::RemoveCredential {
                name: "password:root".to_owned(),
            }),
            MessageDispatchTarget::Storage
        );
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::ActivateTerminalTab {
                session_id: SessionId(uuid::Uuid::nil()),
            }),
            MessageDispatchTarget::Session
        );
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::RefreshSftp {
                host_id: HostId(uuid::Uuid::nil()),
            }),
            MessageDispatchTarget::Sftp
        );
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::OpenShell {
                host_id: HostId(uuid::Uuid::nil()),
            }),
            MessageDispatchTarget::Launch
        );
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::RunCommandHistory {
                history_id: crate::model::CommandHistoryId(uuid::Uuid::nil()),
            }),
            MessageDispatchTarget::Snippet
        );
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::BackendEventReceived(
                BackendEvent::Disconnected {
                    session_id: SessionId(uuid::Uuid::nil()),
                },
            )),
            MessageDispatchTarget::Backend
        );
    }

    #[test]
    fn workspace_page_message_stays_in_ui_target() {
        assert_eq!(
            MessageDispatchTarget::for_message(&Message::SetWorkspacePage {
                page: WorkspacePage::Hosts,
            }),
            MessageDispatchTarget::Ui
        );
    }
}
