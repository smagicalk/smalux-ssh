//! 工作区页面与 Hosts 列表状态。

use crate::model::WorkspacePage;

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};

impl AppState {
    /// 切换当前一级工作区页面。
    pub(in crate::model::app_state) fn set_workspace_page(
        &mut self,
        page: WorkspacePage,
    ) -> AppUpdateOutcome {
        self.ui.workspace.active_page = page;
        draft_changed()
    }

    /// 切换 Hosts 列表展示方式。
    pub(in crate::model::app_state) fn toggle_host_list_mode(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_host_list_mode();
        draft_changed()
    }

    /// 更新 Hosts 面板搜索条件。
    pub(in crate::model::app_state) fn update_host_search_query(
        &mut self,
        query: String,
    ) -> AppUpdateOutcome {
        self.ui.workspace.set_host_search_query(query);
        draft_changed()
    }
}
