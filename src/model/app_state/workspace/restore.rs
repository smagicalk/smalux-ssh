//! 工作区快照恢复。

use crate::model::{SessionKind, SftpBrowserState, WorkspaceState};
use crate::terminal::TerminalTabState;

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 从已保存工作区恢复可见标签页，不自动发起 SSH 连接。
    pub(in crate::model::app_state) fn restore_workspace_snapshot(&mut self) -> AppUpdateOutcome {
        let Some(workspace) = self.storage.workspace.clone() else {
            return AppUpdateOutcome {
                error: Some("没有已保存的工作区快照".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.sessions
            .restore_tabs_from_workspace(&workspace.tabs, workspace.active_tab);
        self.restore_sftp_browsers_from_workspace(&workspace);
        self.restore_terminal_tabs_from_workspace(&workspace);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    fn restore_terminal_tabs_from_workspace(&mut self, workspace: &WorkspaceState) {
        self.terminal.tabs.clear();
        self.terminal.active_tab = None;
        self.terminal.tab_count = 0;

        for tab in &workspace.tabs {
            if matches!(
                tab.kind,
                SessionKind::Shell | SessionKind::RemoteCommand { .. }
            ) {
                self.terminal
                    .open_tab(TerminalTabState::new(tab.session_id, tab.title.clone()));
            }
        }

        if let Some(active_tab) = workspace.active_tab {
            let _ = self.terminal.set_active_tab(active_tab);
        }
    }

    fn restore_sftp_browsers_from_workspace(&mut self, workspace: &WorkspaceState) {
        self.sessions.sftp_browsers.clear();

        for tab in &workspace.tabs {
            if !matches!(tab.kind, SessionKind::Sftp) {
                continue;
            }

            let Some(host_id) = tab.host_id else {
                continue;
            };
            let current_dir = tab
                .working_directory
                .clone()
                .unwrap_or_else(|| "/".to_owned());
            self.sessions.upsert_sftp_browser(SftpBrowserState {
                session_id: tab.session_id,
                host_id,
                current_dir,
                entries: Vec::new(),
                selected_path: None,
                loading: false,
                last_error: None,
            });
        }
    }
}
