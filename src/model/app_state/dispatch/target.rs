use super::super::Message;

#[cfg(test)]
#[path = "target_tests.rs"]
mod tests;

/// `Message` 的领域归属。
///
/// 这个枚举只用于路由，不暴露给 UI。它让 `dispatch.rs` 只关心“送到哪里”，
/// 不关心每个消息的具体参数和业务逻辑。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MessageDispatchTarget {
    /// 主题、背景、主机级视觉覆盖。
    Visual,
    /// 工作区快照保存、恢复和清除。
    Workspace,
    /// 纯 UI 草稿、弹窗、筛选、设置页和终端输入草稿。
    Ui,
    /// 已保存数据的确认删除和本地安全资产管理。
    Storage,
    /// 会话标签页的激活、关闭和运行态清理。
    Session,
    /// SFTP 浏览器、书签和传输操作。
    Sftp,
    /// 打开连接、启动 SFTP、运行命令和隧道。
    Launch,
    /// 命令片段和历史命令执行。
    Snippet,
    /// 后端执行结果或异步事件。
    Backend,
}

impl MessageDispatchTarget {
    /// 返回消息应进入的领域路由。
    ///
    /// 新增消息时必须在这里归类；分类错误会让消息进入错误路由并触发对应
    /// `unreachable!`，因此相关测试会很快暴露问题。
    pub(super) fn for_message(message: &Message) -> Self {
        if is_visual_message(message) {
            Self::Visual
        } else if is_workspace_message(message) {
            Self::Workspace
        } else if is_ui_message(message) {
            Self::Ui
        } else if is_storage_message(message) {
            Self::Storage
        } else if is_session_message(message) {
            Self::Session
        } else if is_sftp_message(message) {
            Self::Sftp
        } else if is_launch_message(message) {
            Self::Launch
        } else if is_snippet_message(message) {
            Self::Snippet
        } else {
            match message {
                Message::BackendEventReceived(_) => Self::Backend,
                _ => unreachable!("所有 Message 变体必须归类到一个 dispatch target"),
            }
        }
    }
}

fn is_visual_message(message: &Message) -> bool {
    matches!(
        message,
        Message::UpdateVisualSettingsDraft { .. }
            | Message::SetVisualBackgroundEnabled { .. }
            | Message::ApplyVisualSettings
            | Message::UpdateHostVisualSettingsDraft { .. }
            | Message::SetHostVisualBackgroundEnabled { .. }
            | Message::ApplyHostVisualSettings { .. }
            | Message::ClearHostVisualSettings { .. }
    )
}

fn is_workspace_message(message: &Message) -> bool {
    matches!(
        message,
        Message::SaveWorkspaceSnapshot
            | Message::RestoreWorkspaceSnapshot
            | Message::ClearWorkspaceSnapshot
    )
}

fn is_ui_message(message: &Message) -> bool {
    matches!(
        message,
        Message::UpdateQuickHostDraft { .. }
            | Message::SelectQuickHostGroup { .. }
            | Message::UpdateQuickHostAuthKind { .. }
            | Message::UpdateQuickHostAuthField { .. }
            | Message::ToggleQuickHostNetworkProxy { .. }
            | Message::ToggleQuickHostNetworkJumpChain { .. }
            | Message::ToggleQuickHostNetworkForward { .. }
            | Message::SaveQuickHost
            | Message::OpenCreateHostDialogInGroup { .. }
            | Message::OpenCreateGroupParentDialog { .. }
            | Message::SelectCreateGroupParent { .. }
            | Message::CloseCreateGroupParentDialog
            | Message::ConfirmCreateGroupParent
            | Message::OpenCreateGroupDialog { .. }
            | Message::UpdateQuickGroupName { .. }
            | Message::SelectQuickGroupParent { .. }
            | Message::CloseCreateGroupDialog
            | Message::SaveQuickGroup
            | Message::DismissUiError
            | Message::OpenCreateHostDialog
            | Message::OpenEditHostDialog { .. }
            | Message::DuplicateHost { .. }
            | Message::CloseCreateHostDialog
            | Message::RequestRemoveHost { .. }
            | Message::CancelRemoveHost
            | Message::RequestRemoveGroup { .. }
            | Message::CancelRemoveGroup
            | Message::SetWorkspacePage { .. }
            | Message::NavigateWorkspacePage { .. }
            | Message::ToggleHostListMode
            | Message::ToggleHostTreeGroup { .. }
            | Message::ToggleCredentialTreeNode { .. }
            | Message::UpdateHostSearchQuery { .. }
            | Message::UpdateCredentialSearchQuery { .. }
            | Message::UpdateSnippetSearchQuery { .. }
            | Message::UpdateNetworkSearchQuery { .. }
            | Message::ToggleSnippetTreeNode { .. }
            | Message::UpdateNewSessionSearchQuery { .. }
            | Message::ResizeHostsPanel { .. }
            | Message::ResizeActivityPanel { .. }
            | Message::ResizeToolPanel { .. }
            | Message::OpenToolPanel { .. }
            | Message::CloseToolPanel
            | Message::ToggleRightSidebar
            | Message::OpenCommandPalette { .. }
            | Message::UpdateCommandPaletteQuery { .. }
            | Message::CloseCommandPalette
            | Message::NextTheme
            | Message::SetLanguage { .. }
            | Message::SetBuiltInTheme { .. }
            | Message::ExportCurrentTheme { .. }
            | Message::CopyCurrentBuiltInTheme
            | Message::ImportTheme { .. }
            | Message::ApplyThemeProfile { .. }
            | Message::RemoveThemeProfile { .. }
            | Message::BackupStorage { .. }
            | Message::ExportStorageSnapshot { .. }
            | Message::ImportStorageSnapshot { .. }
            | Message::ImportSqliteBackup { .. }
            | Message::NextBackground
            | Message::OpenLocalTerminal
            | Message::UpdateTerminalInputDraft { .. }
            | Message::AppendTerminalInputDraft { .. }
            | Message::BackspaceTerminalInputDraft { .. }
            | Message::SendTerminalInput { .. }
            | Message::UpdateHostCommandDraft { .. }
            | Message::UpdateHostSftpInitialDirDraft { .. }
            | Message::UpdateSftpActionDraft { .. }
    )
}

fn is_storage_message(message: &Message) -> bool {
    matches!(
        message,
        Message::ConfirmRemoveHost
            | Message::ConfirmRemoveGroup
            | Message::CreateCredentialGroup { .. }
            | Message::RenameCredentialGroup { .. }
            | Message::RemoveCredentialGroup { .. }
            | Message::CreateCredentialMetadata { .. }
            | Message::UpdateCredentialMetadata { .. }
            | Message::UpdateCredentialSecret { .. }
            | Message::GeneratePrivateKeyCredential { .. }
            | Message::SavePasswordCredential { .. }
            | Message::ImportPrivateKeyCredential { .. }
            | Message::ImportPrivateKeyTextCredential { .. }
            | Message::ImportCertificateCredential { .. }
            | Message::ImportCertificateTextCredential { .. }
            | Message::GenerateCertificateCredential { .. }
            | Message::ExportCredentialSecret { .. }
            | Message::DuplicateCredential { .. }
            | Message::RemoveCredential { .. }
            | Message::MoveCredential { .. }
            | Message::MoveCredentialGroup { .. }
            | Message::SaveProxyAsset { .. }
            | Message::SaveJumpChainAsset { .. }
            | Message::SaveForwardAsset { .. }
            | Message::RemoveProxyAsset { .. }
            | Message::RemoveJumpChainAsset { .. }
            | Message::RemoveForwardAsset { .. }
            | Message::TrustKnownHost { .. }
            | Message::RemoveKnownHost { .. }
    )
}

fn is_session_message(message: &Message) -> bool {
    matches!(
        message,
        Message::CloseSessionTab { .. } | Message::ActivateTerminalTab { .. }
    )
}

fn is_sftp_message(message: &Message) -> bool {
    matches!(
        message,
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
            | Message::CreateSftpDir { .. }
    )
}

fn is_launch_message(message: &Message) -> bool {
    matches!(
        message,
        Message::OpenShell { .. }
            | Message::OpenRecentConnection { .. }
            | Message::ReconnectShell { .. }
            | Message::OpenSftp { .. }
            | Message::RunRemoteCommand { .. }
            | Message::StartTunnel { .. }
            | Message::StopTunnel { .. }
    )
}

fn is_snippet_message(message: &Message) -> bool {
    matches!(
        message,
        Message::SaveHostCommandSnippet { .. }
            | Message::RunSnippet { .. }
            | Message::RunSnippetWithArguments { .. }
            | Message::RunSnippetTargetWithArguments { .. }
            | Message::RunSnippetOnActiveHost { .. }
            | Message::RunSnippetTargetOnActiveHost { .. }
            | Message::UpdateSnippetArgument { .. }
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
            | Message::RemoveSnippet { .. }
            | Message::RunCommandHistory { .. }
    )
}
