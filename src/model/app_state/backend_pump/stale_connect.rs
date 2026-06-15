//! 后端泵中过期连接命令的状态收尾。
//!
//! 连接命令过期通常表示用户在连接完成前关闭了标签页，或目标主机已不再可用。它的后续
//! 命令依赖连接成功，因此这里会同时丢弃同一会话的待执行命令。

use crate::backend::BackendEvent;
use crate::core::CoreState;
use crate::model::SessionId;

use super::super::AppUpdateOutcome;
use super::super::pending::discard_pending_commands_for_failed_session;

impl CoreState {
    pub(super) fn skip_stale_connect_command(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        let reason = "连接命令已失效，后续启动命令未执行".to_owned();
        // 先让会话本身进入失败态，保证 UI 不会卡在连接中。
        let event_outcome = self.apply_backend_event(BackendEvent::Failed {
            session_id,
            reason: reason.clone(),
        });
        // 再清理同会话后续命令，尤其是依赖连接成功的 OpenShell/SFTP 命令。
        let discarded = discard_pending_commands_for_failed_session(
            &mut self.backend_commands,
            session_id,
            &reason,
        );
        let mut outcome = AppUpdateOutcome {
            state_changed: event_outcome.state_changed || discarded.removed_count > 0,
            applied_backend_events: event_outcome.applied_backend_events,
            ..AppUpdateOutcome::default()
        };
        // 被丢弃的传输命令仍需要补失败事件，否则传输列表会残留 running 状态。
        for event in discarded.failure_events {
            let event_outcome = self.apply_backend_event(event);
            outcome.state_changed |= event_outcome.state_changed;
            outcome.applied_backend_events += event_outcome.applied_backend_events;
        }
        outcome
    }
}
