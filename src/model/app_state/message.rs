//! UI 与后台任务之间传递的消息。

use crate::backend::BackendEvent;
use crate::model::{
    CommandHistoryId, HostId, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField,
    SessionId, SftpActionDraftField, SnippetId, ToolPanelMode, TransferId, TunnelRule,
    VisualSettingsDraftField, WorkspacePage,
};

/// UI 与后台任务之间传递的消息。
#[derive(Debug, Clone)]
pub enum Message {
    UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField,
        value: String,
    },
    SetVisualBackgroundEnabled {
        enabled: bool,
    },
    ApplyVisualSettings,
    UpdateHostVisualSettingsDraft {
        host_id: HostId,
        field: VisualSettingsDraftField,
        value: String,
    },
    SetHostVisualBackgroundEnabled {
        host_id: HostId,
        enabled: bool,
    },
    ApplyHostVisualSettings {
        host_id: HostId,
    },
    ClearHostVisualSettings {
        host_id: HostId,
    },
    SaveWorkspaceSnapshot,
    RestoreWorkspaceSnapshot,
    ClearWorkspaceSnapshot,
    UpdateQuickHostDraft {
        field: QuickHostDraftField,
        value: String,
    },
    UpdateQuickHostAuthKind {
        kind: QuickHostAuthKind,
    },
    UpdateQuickHostAuthField {
        field: QuickHostAuthField,
        value: String,
    },
    SaveQuickHost,
    RemoveCredential {
        name: String,
    },
    TrustKnownHost {
        host: String,
        port: u16,
    },
    RemoveKnownHost {
        host: String,
        port: u16,
    },
    DismissUiError,
    SetWorkspacePage {
        page: WorkspacePage,
    },
    ToggleHostListMode,
    UpdateHostSearchQuery {
        query: String,
    },
    ResizeHostsPanel {
        width: i32,
    },
    ResizeActivityPanel {
        width: i32,
    },
    ResizeToolPanel {
        width: i32,
    },
    OpenToolPanel {
        mode: ToolPanelMode,
    },
    CloseToolPanel,
    ToggleRightSidebar,
    OpenCommandPalette {
        query: String,
    },
    UpdateCommandPaletteQuery {
        query: String,
    },
    CloseCommandPalette,
    NextBackground,
    CloseSessionTab {
        session_id: SessionId,
    },
    ActivateTerminalTab {
        session_id: SessionId,
    },
    UpdateTerminalInputDraft {
        session_id: SessionId,
        input: String,
    },
    AppendTerminalInputDraft {
        session_id: SessionId,
        text: String,
    },
    BackspaceTerminalInputDraft {
        session_id: SessionId,
    },
    SendTerminalInput {
        session_id: SessionId,
    },
    UpdateHostCommandDraft {
        host_id: HostId,
        command: String,
    },
    UpdateHostSftpInitialDirDraft {
        host_id: HostId,
        initial_dir: String,
    },
    UpdateSftpActionDraft {
        host_id: HostId,
        field: SftpActionDraftField,
        value: String,
    },
    RefreshSftp {
        host_id: HostId,
    },
    SaveSftpBookmark {
        host_id: HostId,
    },
    OpenSftpBookmark {
        host_id: HostId,
        remote_path: String,
    },
    RemoveSftpBookmark {
        host_id: HostId,
        remote_path: String,
    },
    NavigateSftp {
        host_id: HostId,
        remote_path: String,
    },
    SelectSftpEntry {
        host_id: HostId,
        remote_path: String,
    },
    UploadSftp {
        host_id: HostId,
    },
    DownloadSftp {
        host_id: HostId,
        remote_path: String,
    },
    CancelSftpTransfer {
        transfer_id: TransferId,
    },
    RemoveSftpFile {
        host_id: HostId,
        remote_path: String,
    },
    CreateSftpDir {
        host_id: HostId,
    },
    OpenShell {
        host_id: HostId,
    },
    OpenRecentConnection {
        host_id: HostId,
    },
    OpenSftp {
        host_id: HostId,
        initial_dir: String,
    },
    RunRemoteCommand {
        host_id: HostId,
        command: String,
        request_pty: bool,
    },
    SaveHostCommandSnippet {
        host_id: HostId,
    },
    RunSnippet {
        host_id: HostId,
        snippet_id: SnippetId,
    },
    UpdateSnippetArgument {
        snippet_id: SnippetId,
        name: String,
        value: String,
    },
    RemoveSnippet {
        snippet_id: SnippetId,
    },
    RunCommandHistory {
        history_id: CommandHistoryId,
    },
    StartTunnel {
        host_id: HostId,
        rule: TunnelRule,
    },
    StopTunnel {
        session_id: SessionId,
        rule_name: String,
    },
    BackendEventReceived(BackendEvent),
}
