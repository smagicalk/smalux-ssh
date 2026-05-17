//! 工作区快照保存和恢复。

use crate::model::{
    SessionKind, SessionTab, SftpBrowserState, SplitAxis, WorkspaceState, WorkspaceTabSnapshot,
};
use crate::terminal::TerminalTabState;

use super::{AppState, AppUpdateOutcome};

const DEFAULT_WORKSPACE_NAME: &str = "default";

impl AppState {
    /// 保存当前会话标签页和基础分屏布局。
    pub(super) fn save_workspace_snapshot(&mut self) -> AppUpdateOutcome {
        if self.sessions.tabs.is_empty() {
            return AppUpdateOutcome {
                error: Some("当前没有可保存的会话标签页".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let mut workspace = WorkspaceState::empty(DEFAULT_WORKSPACE_NAME);
        for tab in &self.sessions.tabs {
            workspace.upsert_tab(WorkspaceTabSnapshot {
                session_id: tab.id,
                host_id: tab.host_id,
                kind: tab.kind.clone(),
                title: tab.title.clone(),
                working_directory: self.working_directory_for_tab(tab),
            });
        }
        workspace.active_tab = self.sessions.active_tab;
        workspace.rebuild_linear_layout(SplitAxis::Horizontal);
        self.storage.save_workspace(workspace);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 从已保存工作区恢复可见标签页，不自动发起 SSH 连接。
    pub(super) fn restore_workspace_snapshot(&mut self) -> AppUpdateOutcome {
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

    /// 清除已保存的工作区快照。
    pub(super) fn clear_workspace_snapshot(&mut self) -> AppUpdateOutcome {
        if self.storage.clear_workspace() {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some("没有已保存的工作区快照".to_owned()),
                ..AppUpdateOutcome::default()
            }
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

    fn working_directory_for_tab(&self, tab: &SessionTab) -> Option<String> {
        match tab.kind {
            SessionKind::Sftp => tab.host_id.and_then(|host_id| {
                self.sessions
                    .sftp_browsers
                    .iter()
                    .find(|browser| browser.host_id == host_id)
                    .map(|browser| browser.current_dir.clone())
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthProfile, Host, HostId, Message, SessionId};
    use uuid::Uuid;

    fn host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            tags: Vec::new(),
            address: "prod.example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                key_hint: None,
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn save_workspace_snapshot_records_current_tabs_and_layout() {
        let mut state = AppState::default();
        let host = host();
        let host_id = host.id;
        let session_id = SessionId(Uuid::new_v4());
        state.storage.upsert_host(host);
        state
            .sessions
            .open_shell_tab(session_id, host_id, "production");
        state
            .terminal
            .open_tab(TerminalTabState::new(session_id, "production"));

        let outcome = state.apply(Message::SaveWorkspaceSnapshot);

        assert!(outcome.changed());
        let workspace = state
            .storage
            .workspace
            .as_ref()
            .expect("应该保存工作区快照");
        assert_eq!(workspace.tabs.len(), 1);
        assert_eq!(workspace.active_tab, Some(session_id));
        assert!(workspace.layout.is_some());
    }

    #[test]
    fn restore_workspace_snapshot_rebuilds_session_and_terminal_tabs() {
        let mut state = AppState::default();
        let host = host();
        let host_id = host.id;
        let shell_id = SessionId(Uuid::new_v4());
        let sftp_id = SessionId(Uuid::new_v4());
        let mut workspace = WorkspaceState::empty("restore");
        state.storage.upsert_host(host);
        workspace.upsert_tab(WorkspaceTabSnapshot {
            session_id: shell_id,
            host_id: Some(host_id),
            kind: SessionKind::Shell,
            title: "shell".to_owned(),
            working_directory: None,
        });
        workspace.upsert_tab(WorkspaceTabSnapshot {
            session_id: sftp_id,
            host_id: Some(host_id),
            kind: SessionKind::Sftp,
            title: "SFTP /home/ops".to_owned(),
            working_directory: Some("/home/ops".to_owned()),
        });
        workspace.active_tab = Some(shell_id);
        workspace.rebuild_linear_layout(SplitAxis::Horizontal);
        state.storage.save_workspace(workspace);

        let outcome = state.apply(Message::RestoreWorkspaceSnapshot);

        assert!(outcome.changed());
        assert_eq!(state.sessions.tab_count(), 2);
        assert_eq!(state.terminal.tab_count(), 1);
        assert_eq!(state.sessions.sftp_browser_count(), 1);
        assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/home/ops");
        assert_eq!(state.sessions.active_tab, Some(shell_id));
        assert_eq!(state.terminal.active_tab, Some(shell_id));
    }

    #[test]
    fn clear_workspace_snapshot_removes_saved_workspace() {
        let mut state = AppState::default();
        state
            .storage
            .save_workspace(WorkspaceState::empty("restore"));

        let outcome = state.apply(Message::ClearWorkspaceSnapshot);

        assert!(outcome.changed());
        assert!(state.storage.workspace.is_none());
    }
}
