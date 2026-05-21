//! 工作区命令面板。

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};

impl AppState {
    /// 打开命令面板。
    pub(in crate::model::app_state) fn open_command_palette(
        &mut self,
        query: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.open_command_palette(query);
        draft_changed()
    }

    /// 更新命令面板查询。
    pub(in crate::model::app_state) fn update_command_palette_query(
        &mut self,
        query: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.command_palette.query = query;
        self.ui.workspace.command_palette.open = true;
        draft_changed()
    }

    /// 关闭命令面板。
    pub(in crate::model::app_state) fn close_command_palette(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.close_command_palette();
        draft_changed()
    }
}
