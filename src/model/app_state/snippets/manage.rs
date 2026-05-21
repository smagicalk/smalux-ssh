//! 快捷命令维护。

use crate::model::SnippetId;

use super::super::{AppState, AppUpdateOutcome};
use super::outcome::missing_snippet;

impl AppState {
    /// 删除指定快捷命令。
    pub(in crate::model::app_state) fn remove_snippet(
        &mut self,
        snippet_id: SnippetId,
    ) -> AppUpdateOutcome {
        if self.storage.remove_snippet(snippet_id) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            missing_snippet(snippet_id)
        }
    }

    /// 更新快捷命令变量最近一次输入值。
    pub(in crate::model::app_state) fn update_snippet_argument(
        &mut self,
        snippet_id: SnippetId,
        name: String,
        value: String,
    ) -> AppUpdateOutcome {
        if self
            .storage
            .upsert_snippet_argument(snippet_id, &name, value)
        {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到快捷命令变量：{name}")),
                ..AppUpdateOutcome::default()
            }
        }
    }
}
