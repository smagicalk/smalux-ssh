//! 工作区命令面板状态操作。

use serde::{Deserialize, Serialize};

use super::WorkspaceUiState;

/// 命令面板查询状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
        }
    }
}

impl WorkspaceUiState {
    /// 打开命令面板并设置查询文本。
    pub fn open_command_palette(&mut self, query: impl Into<String>) {
        self.command_palette.open = true;
        self.command_palette.query = query.into();
    }

    /// 关闭命令面板并清空查询。
    pub fn close_command_palette(&mut self) {
        self.command_palette.open = false;
        self.command_palette.query.clear();
    }
}
