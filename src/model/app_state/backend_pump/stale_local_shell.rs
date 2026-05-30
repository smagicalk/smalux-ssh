//! 后端泵中过期本地 shell 启动命令的状态收尾。
//!
//! 本地 shell 没有远程主机依赖，但用户仍可能在 PTY 启动前关闭标签页。这里用统一的
//! `BackendEvent::Failed` 收尾，让终端标签页和普通 SSH shell 保持同一种失败状态。

use crate::backend::BackendEvent;
use crate::model::SessionId;

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn skip_stale_local_shell_command(
        &mut self,
        session_id: SessionId,
    ) -> AppUpdateOutcome {
        // 通过正常后端事件路径更新状态，避免本地终端拥有独立的 UI 特判。
        let event_outcome = self.apply_backend_event(BackendEvent::Failed {
            session_id,
            reason: "本地终端启动命令已失效".to_owned(),
        });

        AppUpdateOutcome {
            state_changed: event_outcome.state_changed,
            applied_backend_events: event_outcome.applied_backend_events,
            ..AppUpdateOutcome::default()
        }
    }
}
