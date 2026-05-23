//! 工作区语言模式展示。

use serde::{Deserialize, Serialize};

use super::WorkspaceUiState;

/// UI 语言选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageMode {
    FollowSystem,
    Chinese,
    English,
}

impl Default for LanguageMode {
    fn default() -> Self {
        Self::FollowSystem
    }
}

impl LanguageMode {
    /// 用于设置页展示的语言模式标签。
    pub fn label(self) -> &'static str {
        match self {
            Self::FollowSystem => "system",
            Self::Chinese => "zh-CN",
            Self::English => "en-US",
        }
    }
}

impl WorkspaceUiState {
    /// 返回当前语言模式标签。
    pub fn language_label(&self) -> &'static str {
        self.language.label()
    }
}
