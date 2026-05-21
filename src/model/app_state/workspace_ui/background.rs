//! 工作区背景轮播。

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};

impl AppState {
    /// 切换到下一张背景轮播图。
    pub(in crate::model::app_state) fn next_background(&mut self) -> AppUpdateOutcome {
        let source_count = self.config.background.normalized().sources.len();
        self.ui.workspace.next_background(source_count);
        draft_changed()
    }
}
