//! 工作区快照清除。

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 清除已保存的工作区快照。
    pub(in crate::model::app_state) fn clear_workspace_snapshot(&mut self) -> AppUpdateOutcome {
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
}
