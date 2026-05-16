//! UI 交互草稿状态。
//!
//! 这里仅保存尚未提交给后端的轻量输入值，避免把输入框状态混入 SSH 会话模型。

mod quick_host;
#[cfg(test)]
mod quick_host_tests;
mod sftp_action;
mod terminal_input;
mod visual_settings;
#[cfg(test)]
mod visual_settings_tests;
mod workspace_ui;

use super::{BackgroundProfile, HostId, ThemeProfile};

pub use quick_host::*;
pub use sftp_action::*;
pub use terminal_input::*;
pub use visual_settings::*;
pub use workspace_ui::*;

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
    pub workspace: WorkspaceUiState,
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
            workspace: WorkspaceUiState::default(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn host_id() -> HostId {
        HostId(Uuid::new_v4())
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
}
