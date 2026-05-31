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
            | Message::SelectQuickHostGroup { .. }
            | Message::UpdateQuickHostAuthKind { .. }
            | Message::UpdateQuickHostAuthField { .. }
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
            | Message::UpdateHostSearchQuery { .. }
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
            | Message::UpdateSftpActionDraft { .. } => Self::Ui,

            Message::ConfirmRemoveHost
            | Message::ConfirmRemoveGroup
            | Message::RemoveCredential { .. }
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
