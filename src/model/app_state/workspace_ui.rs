//! 工作区页面、分栏和命令面板的 UI 消息处理。

use crate::model::{ToolPanelMode, WorkspacePage};

use super::{AppState, AppUpdateOutcome, ui_drafts::draft_changed};

impl AppState {
    /// 切换当前一级工作区页面。
    pub(super) fn set_workspace_page(&mut self, page: WorkspacePage) -> AppUpdateOutcome {
        self.ui.workspace.active_page = page;
        draft_changed()
    }

    /// 切换 Hosts 列表展示方式。
    pub(super) fn toggle_host_list_mode(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_host_list_mode();
        draft_changed()
    }

    /// 更新 Hosts 面板搜索条件。
    pub(super) fn update_host_search_query(&mut self, query: String) -> AppUpdateOutcome {
        self.ui.workspace.set_host_search_query(query);
        draft_changed()
    }

    /// 调整 Hosts 面板宽度。
    pub(super) fn resize_hosts_panel(&mut self, width: i32) -> AppUpdateOutcome {
        let before = self.ui.workspace.hosts_panel_width;
        self.ui.workspace.set_hosts_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.hosts_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 调整右侧活动栏宽度。
    pub(super) fn resize_activity_panel(&mut self, width: i32) -> AppUpdateOutcome {
        let before = self.ui.workspace.activity_panel_width;
        self.ui.workspace.set_activity_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.activity_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 调整 D 区域内部工具/SFTP 分栏宽度。
    pub(super) fn resize_tool_panel(&mut self, width: i32) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_width;
        self.ui.workspace.set_tool_panel_width(width);
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_width,
            ..AppUpdateOutcome::default()
        }
    }

    /// 打开 D 区域内部辅助分栏。
    pub(super) fn open_tool_panel(&mut self, mode: ToolPanelMode) -> AppUpdateOutcome {
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
    pub(super) fn close_tool_panel(&mut self) -> AppUpdateOutcome {
        let before = self.ui.workspace.tool_panel_mode;
        self.ui.workspace.close_tool_panel();
        AppUpdateOutcome {
            state_changed: before != self.ui.workspace.tool_panel_mode,
            ..AppUpdateOutcome::default()
        }
    }

    /// 折叠或展开右侧详情栏。
    pub(super) fn toggle_right_sidebar(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.toggle_right_sidebar();
        draft_changed()
    }

    /// 打开命令面板。
    pub(super) fn open_command_palette(&mut self, query: String) -> AppUpdateOutcome {
        self.ui.workspace.open_command_palette(query);
        draft_changed()
    }

    /// 更新命令面板查询。
    pub(super) fn update_command_palette_query(&mut self, query: String) -> AppUpdateOutcome {
        self.ui.workspace.command_palette.query = query;
        self.ui.workspace.command_palette.open = true;
        draft_changed()
    }

    /// 关闭命令面板。
    pub(super) fn close_command_palette(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.close_command_palette();
        draft_changed()
    }

    /// 切换到下一张背景轮播图。
    pub(super) fn next_background(&mut self) -> AppUpdateOutcome {
        let source_count = self.config.background.normalized().sources.len();
        self.ui.workspace.next_background(source_count);
        draft_changed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HostId, Message, SessionId};
    use crate::terminal::TerminalTabState;
    use uuid::Uuid;

    #[test]
    fn workspace_ui_messages_update_layout_state_only() {
        let mut state = AppState::default();

        state.apply(Message::SetWorkspacePage {
            page: WorkspacePage::Settings,
        });
        state.apply(Message::ToggleHostListMode);
        state.apply(Message::UpdateHostSearchQuery {
            query: "prod".to_owned(),
        });
        state.apply(Message::ResizeHostsPanel { width: 260 });
        state.apply(Message::ResizeActivityPanel { width: 300 });
        state.apply(Message::ResizeToolPanel { width: 360 });
        state.apply(Message::OpenToolPanel {
            mode: ToolPanelMode::History,
        });
        state.apply(Message::ToggleRightSidebar);
        state.apply(Message::OpenCommandPalette {
            query: "prod".to_owned(),
        });

        assert_eq!(state.ui.workspace.active_page, WorkspacePage::Settings);
        assert!(matches!(
            state.ui.workspace.host_list_mode,
            crate::model::HostListMode::Card
        ));
        assert_eq!(state.ui.workspace.host_search_query, "prod");
        assert_eq!(state.ui.workspace.hosts_panel_width, 260);
        assert_eq!(state.ui.workspace.activity_panel_width, 300);
        assert_eq!(state.ui.workspace.tool_panel_width, 360);
        assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::History);
        assert!(state.ui.workspace.right_sidebar_collapsed);
        assert!(state.ui.workspace.command_palette.open);
        assert_eq!(state.ui.workspace.command_palette.query, "prod");
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn tool_panel_messages_update_layout_state_only() {
        let mut state = AppState::default();

        let open = state.apply(Message::OpenToolPanel {
            mode: ToolPanelMode::History,
        });
        let close = state.apply(Message::CloseToolPanel);

        assert!(open.changed());
        assert!(close.changed());
        assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::Closed);
        assert_eq!(state.backend_commands.pending_count(), 0);
    }

    #[test]
    fn opening_sftp_tool_panel_returns_to_terminal_workspace() {
        let mut state = AppState::default();
        let host_id = HostId(Uuid::new_v4());
        let session_id = SessionId(Uuid::new_v4());
        state
            .sessions
            .open_shell_tab(session_id, host_id, "production");
        state
            .terminal
            .open_tab(TerminalTabState::new(session_id, "production"));
        state.ui.workspace.active_page = WorkspacePage::Sftp;

        let open = state.apply(Message::OpenToolPanel {
            mode: ToolPanelMode::Sftp,
        });

        assert!(open.changed());
        assert_eq!(state.ui.workspace.active_page, WorkspacePage::Terminal);
        assert_eq!(state.ui.workspace.tool_panel_mode, ToolPanelMode::Sftp);
        assert_eq!(state.sessions.active_tab, Some(session_id));
    }
}
