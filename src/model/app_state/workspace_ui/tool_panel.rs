//! 工作区 D 区域内部辅助分栏。

use crate::model::{ToolPanelMode, WorkspacePage};

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 打开 D 区域内部辅助分栏。
    pub(in crate::model::app_state) fn open_tool_panel(
        &mut self,
        mode: ToolPanelMode,
    ) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_mode;
        let before_page = self.ui.workspace.active_page;
        let before_active_tab = self.sessions.active_tab;
        self.ui.workspace.open_tool_panel(mode);
        if matches!(mode, ToolPanelMode::Sftp) {
            self.ui.workspace.active_page = WorkspacePage::Terminal;
            if let Some(active_terminal) = self.terminal.active_tab {
                self.sessions.active_tab = Some(active_terminal);
            }
        }
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_mode
                || before_page != self.ui.workspace.active_page
                || before_active_tab != self.sessions.active_tab,
            ..AppUpdateOutcome::default()
        }
    }

    /// 关闭 D 区域内部辅助分栏。
    pub(in crate::model::app_state) fn close_tool_panel(&mut self) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_mode;
        self.ui.workspace.close_tool_panel();
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_mode,
            ..AppUpdateOutcome::default()
        }
    }
}
