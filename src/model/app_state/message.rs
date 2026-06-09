//! UI 与后台任务之间传递的消息。
//!
//! `Message` 是核心层的操作语言。它不携带 Slint 类型，也不依赖具体窗口框架。
//! 当前 Slint UI、未来 Web UI 或命令行 UI 都应该先把自己的事件解析成这里的
//! 消息，再交给 `AppState::apply`。
//!
//! 约定：
//!
//! - UI 字符串输入在进入消息前尽量转成核心 ID 或枚举，例如 `HostId`、
//!   `GroupId`、`WorkspacePage`。
//! - 消息只描述意图，不直接执行网络或文件操作；需要后端执行的工作会被
//!   `AppState` 转成后端命令队列。
//! - 如果新增 UI 功能，优先新增明确的消息，而不是让 UI 直接改 `AppState`
//!   的内部字段。

use crate::backend::BackendEvent;
use crate::model::{
    BuiltInTheme, CommandHistoryId, CredentialGroupId, CredentialKind, GroupId, HostId,
    KeyAlgorithm, LanguageMode, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField,
    SessionId, SftpActionDraftField, SnippetArgument, SnippetGroupId, SnippetId, SnippetScope,
    SnippetSupportTargetId, ToolPanelMode, TransferId, TunnelRule, VisualSettingsDraftField,
    WorkspacePage,
};
use crate::theme::ThemeExchangeFormat;

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
    SelectQuickHostGroup {
        group_id: Option<GroupId>,
    },
    UpdateQuickHostAuthKind {
        kind: QuickHostAuthKind,
    },
    UpdateQuickHostAuthField {
        field: QuickHostAuthField,
        value: String,
    },
    SaveQuickHost,
    OpenCreateHostDialogInGroup {
        group_id: Option<GroupId>,
    },
    OpenCreateGroupParentDialog {
        parent_id: Option<GroupId>,
    },
    SelectCreateGroupParent {
        parent_id: Option<GroupId>,
    },
    CloseCreateGroupParentDialog,
    ConfirmCreateGroupParent,
    OpenCreateGroupDialog {
        parent_id: Option<GroupId>,
    },
    UpdateQuickGroupName {
        name: String,
    },
    SelectQuickGroupParent {
        parent_id: Option<GroupId>,
    },
    CloseCreateGroupDialog,
    SaveQuickGroup,
    CreateCredentialGroup {
        name: String,
        kind: CredentialKind,
        parent_id: Option<CredentialGroupId>,
    },
    RenameCredentialGroup {
        group_id: CredentialGroupId,
        name: String,
    },
    RemoveCredentialGroup {
        group_id: CredentialGroupId,
    },
    CreateCredentialMetadata {
        kind: CredentialKind,
        name: String,
        group_id: Option<CredentialGroupId>,
        secret_ref: String,
        algorithm: Option<KeyAlgorithm>,
    },
    UpdateCredentialMetadata {
        original_name: String,
        name: String,
        group_id: Option<CredentialGroupId>,
        algorithm: Option<KeyAlgorithm>,
    },
    UpdateCredentialSecret {
        name: String,
        secret_text: String,
    },
    GeneratePrivateKeyCredential {
        name: String,
        group_id: Option<CredentialGroupId>,
        algorithm: Option<KeyAlgorithm>,
    },
    SavePasswordCredential {
        name: String,
        group_id: Option<CredentialGroupId>,
        password: String,
    },
    ImportPrivateKeyCredential {
        name: String,
        group_id: Option<CredentialGroupId>,
        source_path: String,
        algorithm: Option<KeyAlgorithm>,
    },
    ImportPrivateKeyTextCredential {
        name: String,
        group_id: Option<CredentialGroupId>,
        private_key_text: String,
        algorithm: Option<KeyAlgorithm>,
    },
    ImportCertificateCredential {
        name: String,
        group_id: Option<CredentialGroupId>,
        source_path: String,
        algorithm: Option<KeyAlgorithm>,
    },
    ImportCertificateTextCredential {
        name: String,
        group_id: Option<CredentialGroupId>,
        certificate_text: String,
        algorithm: Option<KeyAlgorithm>,
    },
    GenerateCertificateCredential {
        name: String,
        group_id: Option<CredentialGroupId>,
        ca_private_key_ref: String,
        subject_private_key_ref: String,
        cert_type: String,
        principals: String,
        valid_days: String,
        key_id: String,
        serial: String,
    },
    ExportCredentialSecret {
        name: String,
        target_path: String,
    },
    DuplicateCredential {
        name: String,
    },
    RemoveCredential {
        name: String,
    },
    MoveCredential {
        name: String,
        group_id: Option<CredentialGroupId>,
    },
    MoveCredentialGroup {
        group_id: CredentialGroupId,
        parent_id: Option<CredentialGroupId>,
    },
    UpdateSnippetSearchQuery {
        query: String,
    },
    ToggleSnippetTreeNode {
        node_id: String,
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
    OpenCreateHostDialog,
    OpenEditHostDialog {
        host_id: HostId,
    },
    DuplicateHost {
        host_id: HostId,
    },
    CloseCreateHostDialog,
    RequestRemoveHost {
        host_id: HostId,
    },
    CancelRemoveHost,
    ConfirmRemoveHost,
    RequestRemoveGroup {
        group_id: GroupId,
    },
    CancelRemoveGroup,
    ConfirmRemoveGroup,
    SetWorkspacePage {
        page: WorkspacePage,
    },
    NavigateWorkspacePage {
        page: WorkspacePage,
    },
    ToggleHostListMode,
    ToggleHostTreeGroup {
        group_id: Option<GroupId>,
    },
    ToggleCredentialTreeNode {
        node_id: String,
    },
    UpdateHostSearchQuery {
        query: String,
    },
    UpdateCredentialSearchQuery {
        query: String,
    },
    UpdateNewSessionSearchQuery {
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
    NextTheme,
    SetLanguage {
        language: LanguageMode,
    },
    SetBuiltInTheme {
        theme: BuiltInTheme,
    },
    ExportCurrentTheme {
        target_path: String,
        format: ThemeExchangeFormat,
    },
    CopyCurrentBuiltInTheme,
    ImportTheme {
        source_path: String,
    },
    ApplyThemeProfile {
        name: String,
    },
    RemoveThemeProfile {
        name: String,
    },
    BackupStorage {
        target_path: String,
    },
    ExportStorageSnapshot {
        target_path: String,
    },
    ImportStorageSnapshot {
        source_path: String,
    },
    ImportSqliteBackup {
        source_path: String,
    },
    NextBackground,
    OpenLocalTerminal,
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
    ReconnectShell {
        session_id: SessionId,
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
    RunSnippetWithArguments {
        host_id: HostId,
        snippet_id: SnippetId,
        arguments: Vec<SnippetArgument>,
    },
    RunSnippetTargetWithArguments {
        host_id: HostId,
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
        arguments: Vec<SnippetArgument>,
    },
    RunSnippetOnActiveHost {
        snippet_id: SnippetId,
    },
    RunSnippetTargetOnActiveHost {
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
    },
    UpdateSnippetArgument {
        snippet_id: SnippetId,
        name: String,
        value: String,
    },
    CreateSnippet {
        name: String,
        description: String,
        command_template: String,
        scope: SnippetScope,
        group_id: Option<SnippetGroupId>,
    },
    UpdateSnippet {
        snippet_id: SnippetId,
        name: String,
        description: String,
        command_template: String,
        scope: SnippetScope,
        group_id: Option<SnippetGroupId>,
    },
    CreateSnippetTarget {
        snippet_id: SnippetId,
        target_keys: Vec<String>,
        display_name: String,
        command_template: String,
        share_target_id: Option<SnippetSupportTargetId>,
    },
    UpdateSnippetTarget {
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
        target_key: String,
        display_name: String,
        command_template: String,
    },
    RemoveSnippetTarget {
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
    },
    SplitSnippetTargetImplementation {
        snippet_id: SnippetId,
        target_id: SnippetSupportTargetId,
    },
    CreateSnippetGroup {
        name: String,
        parent_id: Option<SnippetGroupId>,
    },
    RenameSnippetGroup {
        group_id: SnippetGroupId,
        name: String,
    },
    RemoveSnippetGroup {
        group_id: SnippetGroupId,
    },
    RemoveSnippetGroupRecursive {
        group_id: SnippetGroupId,
    },
    MoveSnippetGroup {
        group_id: SnippetGroupId,
        parent_id: Option<SnippetGroupId>,
    },
    MoveSnippet {
        snippet_id: SnippetId,
        group_id: Option<SnippetGroupId>,
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
