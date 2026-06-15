//! 工作区快照清除。

use crate::core::CoreState;

use super::super::AppUpdateOutcome;

impl CoreState {
    /// 清除工作区快照的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn clear_workspace_snapshot_action(&mut self) -> AppUpdateOutcome {
        self.clear_workspace_snapshot()
    }

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
