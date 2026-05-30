//! 凭据元数据管理。
//!
//! 这里管理的是“凭据索引/元数据”，不是私钥或密码本体。后续接入加密存储时，真正敏感
//! 数据应在 storage/security 层处理，状态层只通过名称或 ID 发起管理动作。

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 删除一个已保存的凭据元数据。
    pub(in crate::model::app_state) fn remove_credential(
        &mut self,
        name: &str,
    ) -> AppUpdateOutcome {
        // 删除失败时返回用户可见错误，不静默忽略，方便设置页提示配置已经过期。
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
