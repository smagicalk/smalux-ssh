//! 启动流程共享结果构造。

use crate::model::HostId;

use super::super::AppUpdateOutcome;

pub(in crate::model::app_state) fn queued_outcome(
    queued_backend_commands: usize,
) -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        queued_backend_commands,
        ..AppUpdateOutcome::default()
    }
}

pub(in crate::model::app_state) fn missing_host(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}
