//! 工作区分栏尺寸与侧栏状态。

use super::super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};

impl AppState {
    /// 调整 Hosts 面板宽度。
    pub(in crate::model::app_state) fn resize_hosts_panel(
        &mut self,
        width: i32,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.hosts_panel_width;
        self.ui.workspace.set_hosts_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.hosts_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 调整右侧活动栏宽度。
    pub(in crate::model::app_state) fn resize_activity_panel(
        &mut self,
        width: i32,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.activity_panel_width;
        self.ui.workspace.set_activity_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.activity_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 调整 D 区域内部工具/SFTP 分栏宽度。
    pub(in crate::model::app_state) fn resize_tool_panel(
        &mut self,
        width: i32,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_width;
        self.ui.workspace.set_tool_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 折叠或展开右侧详情栏。
    pub(in crate::model::app_state) fn toggle_right_sidebar(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_right_sidebar();
        draft_changed()
    }
}
