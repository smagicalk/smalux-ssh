//! UI 交互草稿状态。
//!
//! 这里仅保存尚未提交给后端的轻量输入值，避免把输入框状态混入 SSH 会话模型。

mod host_action;
mod quick_host;
#[cfg(test)]
mod quick_host_tests;
mod sftp_action;
mod terminal_input;
#[cfg(test)]
mod tests;
mod visual_settings;
#[cfg(test)]
mod visual_settings_tests;
mod workspace_ui;

use super::{BackgroundProfile, GroupId, HostId, ThemeProfile};

pub use host_action::*;
pub use quick_host::*;
pub use sftp_action::*;
pub use terminal_input::*;
pub use visual_settings::*;
pub use workspace_ui::*;

/// 纯 UI 层运行态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiState {
    pub last_error: Option<String>,
    pub quick_host: QuickHostDraft,
    pub quick_group: QuickGroupDraft,
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
            quick_group: QuickGroupDraft::default(),
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
}

/// 快速新增分组表单草稿。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuickGroupDraft {
    pub parent_id: Option<GroupId>,
    pub name: String,
}

impl QuickGroupDraft {
    pub fn with_parent(parent_id: Option<GroupId>) -> Self {
        Self {
            parent_id,
            name: String::new(),
        }
    }
}
