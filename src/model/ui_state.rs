//! UI 交互草稿状态。
//!
//! 这里仅保存尚未提交给后端的轻量输入值，避免把输入框状态混入 SSH 会话模型。

mod quick_host;
mod sftp_action;
mod terminal_input;
mod visual_settings;

use super::{BackgroundProfile, HostId, SessionId, ThemeProfile};

pub use quick_host::*;
pub use sftp_action::*;
pub use terminal_input::*;
pub use visual_settings::*;

pub const DEFAULT_REMOTE_COMMAND: &str = "uptime";
pub const DEFAULT_SFTP_INITIAL_DIR: &str = "/";

/// 每台主机在操作区的输入草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostActionDraft {
    pub host_id: HostId,
    pub remote_command: String,
    pub sftp_initial_dir: String,
}

impl HostActionDraft {
    /// 为主机创建可直接使用的默认操作草稿。
    pub fn new(host_id: HostId) -> Self {
        Self {
            host_id,
            remote_command: DEFAULT_REMOTE_COMMAND.to_owned(),
            sftp_initial_dir: DEFAULT_SFTP_INITIAL_DIR.to_owned(),
        }
    }
}

/// 纯 UI 层运行态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiState {
    pub last_error: Option<String>,
    pub quick_host: QuickHostDraft,
    pub visual_settings: VisualSettingsDraft,
    pub host_visual_settings_drafts: Vec<HostVisualSettingsDraft>,
    pub host_action_drafts: Vec<HostActionDraft>,
    pub sftp_action_drafts: Vec<SftpActionDraft>,
    pub terminal_input_drafts: Vec<TerminalInputDraft>,
}

impl UiState {
    /// 使用当前视觉配置初始化 UI 草稿。
    pub fn from_visual(theme: &ThemeProfile, background: &BackgroundProfile) -> Self {
        Self {
            last_error: None,
            quick_host: QuickHostDraft::default(),
            visual_settings: VisualSettingsDraft::from_profiles(theme, background),
            host_visual_settings_drafts: Vec::new(),
            host_action_drafts: Vec::new(),
            sftp_action_drafts: Vec::new(),
            terminal_input_drafts: Vec::new(),
        }
    }

    /// 记录最近一次需要展示给用户的错误。
    pub fn set_last_error(&mut self, error: impl Into<String>) -> bool {
        let error = error.into();
        if self.last_error.as_deref() == Some(error.as_str()) {
            return false;
        }

        self.last_error = Some(error);
        true
    }

    /// 清除当前错误提示。
    pub fn clear_last_error(&mut self) -> bool {
        let had_error = self.last_error.is_some();
        self.last_error = None;
        had_error
    }

    /// 更新全局视觉配置草稿字段。
    pub fn set_visual_settings_field(
        &mut self,
        field: VisualSettingsDraftField,
        value: impl Into<String>,
    ) {
        self.visual_settings.set_field(field, value);
    }

    /// 更新全局背景开关草稿。
    pub fn set_visual_background_enabled(&mut self, enabled: bool) {
        self.visual_settings.set_background_enabled(enabled);
    }

    /// 返回指定主机的视觉配置草稿。
    pub fn host_visual_settings_for(&self, host_id: HostId) -> Option<&VisualSettingsDraft> {
        self.host_visual_settings_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| &draft.settings)
    }

    /// 准备指定主机的视觉配置草稿。
    pub fn ensure_host_visual_settings_draft(
        &mut self,
        host_id: HostId,
        theme: &ThemeProfile,
        background: &BackgroundProfile,
    ) -> &mut VisualSettingsDraft {
        if let Some(index) = self
            .host_visual_settings_drafts
            .iter()
            .position(|draft| draft.host_id == host_id)
        {
            return &mut self.host_visual_settings_drafts[index].settings;
        }

        self.host_visual_settings_drafts
            .push(HostVisualSettingsDraft {
                host_id,
                settings: VisualSettingsDraft::from_profiles(theme, background),
            });
        &mut self
            .host_visual_settings_drafts
            .last_mut()
            .expect("刚插入的主机视觉草稿应该存在")
            .settings
    }

    /// 更新指定主机的视觉配置草稿字段。
    pub fn set_host_visual_settings_field(
        &mut self,
        host_id: HostId,
        field: VisualSettingsDraftField,
        value: impl Into<String>,
        fallback_theme: &ThemeProfile,
        fallback_background: &BackgroundProfile,
    ) {
        self.ensure_host_visual_settings_draft(host_id, fallback_theme, fallback_background)
            .set_field(field, value);
    }

    /// 更新指定主机的背景开关草稿。
    pub fn set_host_visual_background_enabled(
        &mut self,
        host_id: HostId,
        enabled: bool,
        fallback_theme: &ThemeProfile,
        fallback_background: &BackgroundProfile,
    ) {
        self.ensure_host_visual_settings_draft(host_id, fallback_theme, fallback_background)
            .set_background_enabled(enabled);
    }

    /// 清除指定主机的视觉配置草稿。
    pub fn clear_host_visual_settings_draft(&mut self, host_id: HostId) {
        self.host_visual_settings_drafts
            .retain(|draft| draft.host_id != host_id);
    }

    /// 返回指定主机的远程命令草稿；没有草稿时使用默认命令。
    pub fn remote_command_for(&self, host_id: HostId) -> &str {
        self.host_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.remote_command.as_str())
            .unwrap_or(DEFAULT_REMOTE_COMMAND)
    }

    /// 返回指定主机的 SFTP 初始路径草稿；没有草稿时使用根目录。
    pub fn sftp_initial_dir_for(&self, host_id: HostId) -> &str {
        self.host_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.sftp_initial_dir.as_str())
            .unwrap_or(DEFAULT_SFTP_INITIAL_DIR)
    }

    /// 更新远程命令输入草稿。
    pub fn set_remote_command(&mut self, host_id: HostId, command: impl Into<String>) {
        self.ensure_host_action_draft(host_id).remote_command = command.into();
    }

    /// 更新 SFTP 初始路径输入草稿。
    pub fn set_sftp_initial_dir(&mut self, host_id: HostId, initial_dir: impl Into<String>) {
        self.ensure_host_action_draft(host_id).sftp_initial_dir = initial_dir.into();
    }

    /// 返回指定主机的 SFTP 本地路径草稿；没有草稿时返回空字符串。
    pub fn sftp_local_path_for(&self, host_id: HostId) -> &str {
        self.sftp_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.local_path.as_str())
            .unwrap_or("")
    }

    /// 返回指定主机的 SFTP 远程文件名草稿；没有草稿时返回空字符串。
    pub fn sftp_remote_name_for(&self, host_id: HostId) -> &str {
        self.sftp_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.remote_name.as_str())
            .unwrap_or("")
    }

    /// 返回指定主机的新目录名草稿；没有草稿时返回空字符串。
    pub fn sftp_new_dir_name_for(&self, host_id: HostId) -> &str {
        self.sftp_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.new_dir_name.as_str())
            .unwrap_or("")
    }

    /// 更新指定主机的 SFTP 操作草稿。
    pub fn set_sftp_action_field(
        &mut self,
        host_id: HostId,
        field: SftpActionDraftField,
        value: impl Into<String>,
    ) {
        let value = value.into();

        match field {
            SftpActionDraftField::LocalPath => {
                self.ensure_sftp_action_draft(host_id).local_path = value
            }
            SftpActionDraftField::RemoteName => {
                self.ensure_sftp_action_draft(host_id).remote_name = value
            }
            SftpActionDraftField::NewDirName => {
                self.ensure_sftp_action_draft(host_id).new_dir_name = value
            }
        }
    }

    /// 返回指定终端会话的输入草稿；没有草稿时返回空字符串。
    pub fn terminal_input_for(&self, session_id: SessionId) -> &str {
        self.terminal_input_drafts
            .iter()
            .find(|draft| draft.session_id == session_id)
            .map(|draft| draft.input.as_str())
            .unwrap_or("")
    }

    /// 更新指定终端会话的输入草稿。
    pub fn set_terminal_input(&mut self, session_id: SessionId, input: impl Into<String>) {
        self.ensure_terminal_input_draft(session_id).input = input.into();
    }

    /// 清空指定终端会话的输入草稿。
    pub fn clear_terminal_input(&mut self, session_id: SessionId) {
        self.terminal_input_drafts
            .retain(|draft| draft.session_id != session_id);
    }

    /// 更新快速新增主机表单字段。
    pub fn set_quick_host_field(&mut self, field: QuickHostDraftField, value: impl Into<String>) {
        let value = value.into();

        match field {
            QuickHostDraftField::Name => self.quick_host.name = value,
            QuickHostDraftField::Address => self.quick_host.address = value,
            QuickHostDraftField::Port => self.quick_host.port = value,
            QuickHostDraftField::Username => self.quick_host.username = value,
            QuickHostDraftField::Tags => self.quick_host.tags = value,
        }
    }

    /// 更新快速新增主机的认证方式。
    pub fn set_quick_host_auth_kind(&mut self, kind: QuickHostAuthKind) {
        self.quick_host.auth.kind = kind;
    }

    /// 更新快速新增主机的认证字段。
    pub fn set_quick_host_auth_field(
        &mut self,
        field: QuickHostAuthField,
        value: impl Into<String>,
    ) {
        let value = value.into();

        match field {
            QuickHostAuthField::PasswordSecretRef => {
                self.quick_host.auth.password_secret_ref = value
            }
            QuickHostAuthField::PrivateKeyRef => self.quick_host.auth.private_key_ref = value,
            QuickHostAuthField::PassphraseRef => self.quick_host.auth.passphrase_ref = value,
            QuickHostAuthField::KeyHint => self.quick_host.auth.key_hint = value,
            QuickHostAuthField::CertificateRef => self.quick_host.auth.certificate_ref = value,
        }
    }

    /// 清空快速新增主机表单，保留默认 SSH 端口。
    pub fn reset_quick_host(&mut self) {
        self.quick_host = QuickHostDraft::default();
    }

    fn ensure_host_action_draft(&mut self, host_id: HostId) -> &mut HostActionDraft {
        if let Some(index) = self
            .host_action_drafts
            .iter()
            .position(|draft| draft.host_id == host_id)
        {
            return &mut self.host_action_drafts[index];
        }

        self.host_action_drafts.push(HostActionDraft::new(host_id));
        self.host_action_drafts
            .last_mut()
            .expect("刚插入的主机操作草稿应该存在")
    }

    fn ensure_terminal_input_draft(&mut self, session_id: SessionId) -> &mut TerminalInputDraft {
        if let Some(index) = self
            .terminal_input_drafts
            .iter()
            .position(|draft| draft.session_id == session_id)
        {
            return &mut self.terminal_input_drafts[index];
        }

        self.terminal_input_drafts
            .push(TerminalInputDraft::new(session_id));
        self.terminal_input_drafts
            .last_mut()
            .expect("刚插入的终端输入草稿应该存在")
    }

    fn ensure_sftp_action_draft(&mut self, host_id: HostId) -> &mut SftpActionDraft {
        if let Some(index) = self
            .sftp_action_drafts
            .iter()
            .position(|draft| draft.host_id == host_id)
        {
            return &mut self.sftp_action_drafts[index];
        }

        self.sftp_action_drafts.push(SftpActionDraft::new(host_id));
        self.sftp_action_drafts
            .last_mut()
            .expect("刚插入的 SFTP 操作草稿应该存在")
    }
}

/// 单台主机的视觉配置草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostVisualSettingsDraft {
    pub host_id: HostId,
    pub settings: VisualSettingsDraft,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn host_id() -> HostId {
        HostId(Uuid::new_v4())
    }

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    fn theme() -> ThemeProfile {
        ThemeProfile {
            name: "Default Dark".to_owned(),
            font_family: "JetBrains Mono".to_owned(),
            font_size: 14.0,
        }
    }

    fn background() -> BackgroundProfile {
        BackgroundProfile {
            enabled: false,
            sources: Vec::new(),
            rotation_interval_secs: 300,
            opacity: 0.18,
            blur: 8.0,
        }
    }

    #[test]
    fn sftp_action_drafts_are_scoped_per_host() {
        let mut ui = UiState::default();
        let first = host_id();
        let second = host_id();

        ui.set_sftp_action_field(first, SftpActionDraftField::LocalPath, "C:/tmp/app.tar.gz");
        ui.set_sftp_action_field(second, SftpActionDraftField::RemoteName, "app.tar.gz");
        ui.set_sftp_action_field(second, SftpActionDraftField::NewDirName, "releases");

        assert_eq!(ui.sftp_local_path_for(first), "C:/tmp/app.tar.gz");
        assert_eq!(ui.sftp_remote_name_for(first), "");
        assert_eq!(ui.sftp_new_dir_name_for(second), "releases");
        assert_eq!(ui.sftp_action_drafts.len(), 2);
    }

    #[test]
    fn default_values_are_actionable_without_saved_draft() {
        let ui = UiState::default();
        let host_id = host_id();

        assert_eq!(ui.remote_command_for(host_id), "uptime");
        assert_eq!(ui.sftp_initial_dir_for(host_id), "/");
        assert!(ui.last_error.is_none());
    }

    #[test]
    fn last_error_can_be_set_and_cleared() {
        let mut ui = UiState::default();

        assert!(ui.set_last_error("连接失败"));
        assert_eq!(ui.last_error.as_deref(), Some("连接失败"));
        assert!(!ui.set_last_error("连接失败"));
        assert!(ui.clear_last_error());
        assert!(ui.last_error.is_none());
        assert!(!ui.clear_last_error());
    }

    #[test]
    fn host_action_drafts_are_scoped_per_host() {
        let mut ui = UiState::default();
        let first = host_id();
        let second = host_id();

        ui.set_remote_command(first, "df -h");
        ui.set_sftp_initial_dir(second, "/var/log");

        assert_eq!(ui.remote_command_for(first), "df -h");
        assert_eq!(ui.sftp_initial_dir_for(first), "/");
        assert_eq!(ui.remote_command_for(second), "uptime");
        assert_eq!(ui.sftp_initial_dir_for(second), "/var/log");
        assert_eq!(ui.host_action_drafts.len(), 2);
    }

    #[test]
    fn quick_host_field_updates_and_reset_are_scoped_to_form() {
        let mut ui = UiState::default();

        ui.set_quick_host_field(QuickHostDraftField::Address, "example.com");
        ui.set_quick_host_field(QuickHostDraftField::Username, "ops");
        ui.set_quick_host_auth_kind(QuickHostAuthKind::Password);
        ui.set_quick_host_auth_field(QuickHostAuthField::PasswordSecretRef, "password:ops");
        ui.reset_quick_host();

        assert_eq!(ui.quick_host.address, "");
        assert_eq!(ui.quick_host.username, "");
        assert_eq!(ui.quick_host.port, "22");
        assert!(matches!(ui.quick_host.auth.kind, QuickHostAuthKind::Agent));
    }

    #[test]
    fn terminal_input_drafts_are_scoped_per_session() {
        let mut ui = UiState::default();
        let first = session_id();
        let second = session_id();

        ui.set_terminal_input(first, "ls");
        ui.set_terminal_input(second, "pwd");

        assert_eq!(ui.terminal_input_for(first), "ls");
        assert_eq!(ui.terminal_input_for(second), "pwd");
        assert_eq!(ui.terminal_input_for(session_id()), "");
        assert_eq!(ui.terminal_input_drafts.len(), 2);

        ui.clear_terminal_input(first);
        assert_eq!(ui.terminal_input_for(first), "");
        assert_eq!(ui.terminal_input_for(second), "pwd");
    }

    #[test]
    fn host_visual_settings_drafts_are_scoped_per_host() {
        let mut ui = UiState::default();
        let first = host_id();
        let second = host_id();

        ui.set_host_visual_settings_field(
            first,
            VisualSettingsDraftField::ThemeName,
            "Prod Dark",
            &theme(),
            &background(),
        );
        ui.set_host_visual_background_enabled(second, true, &theme(), &background());

        assert_eq!(
            ui.host_visual_settings_for(first)
                .map(|draft| draft.theme_name.as_str()),
            Some("Prod Dark")
        );
        assert_eq!(
            ui.host_visual_settings_for(second)
                .map(|draft| draft.background_enabled),
            Some(true)
        );
        assert_eq!(ui.host_visual_settings_drafts.len(), 2);

        ui.clear_host_visual_settings_draft(first);
        assert!(ui.host_visual_settings_for(first).is_none());
        assert!(ui.host_visual_settings_for(second).is_some());
    }
}
