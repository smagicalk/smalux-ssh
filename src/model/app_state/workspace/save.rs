//! 工作区快照保存。

use crate::model::{SessionKind, SessionTab, SplitAxis, WorkspaceState, WorkspaceTabSnapshot};

use crate::core::CoreState;

use super::super::AppUpdateOutcome;
use super::DEFAULT_WORKSPACE_NAME;

impl CoreState {
    /// 保存工作区快照的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn save_workspace_snapshot_action(&mut self) -> AppUpdateOutcome {
        self.save_workspace_snapshot()
    }

    /// 保存当前会话标签页和基础分屏布局。
    pub(in crate::model::app_state) fn save_workspace_snapshot(&mut self) -> AppUpdateOutcome {
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
