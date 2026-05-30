//! 后端泵中过期远程命令的状态收尾。
//!
//! 远程命令过期时后端还没有执行真实命令，因此没有 exit code。这里只把命令历史中的运行
//! 状态结束掉，避免历史列表一直显示为未完成。

use crate::model::SessionId;

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn skip_stale_remote_command(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        AppUpdateOutcome {
            // None 表示没有可用退出码，历史条目只做结束时间/状态收尾。
            state_changed: self.finish_remote_command_history(session_id, None),
            ..AppUpdateOutcome::default()
        }
    }
}
