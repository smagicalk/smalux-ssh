//! 凭据元数据管理。

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 删除一个已保存的凭据元数据。
    pub(in crate::model::app_state) fn remove_credential(
        &mut self,
        name: &str,
    ) -> AppUpdateOutcome {
        if self.storage.remove_credential(name) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到凭据：{name}")),
                ..AppUpdateOutcome::default()
            }
        }
    }
}
