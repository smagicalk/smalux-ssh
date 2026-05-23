//! 主机连接状态展示标签。

use crate::model::{AppState, HostId, SessionStatus};

pub(super) fn host_status_label(state: &AppState, host_id: HostId) -> &'static str {
    let status = state
        .sessions
        .tabs
        .iter()
        .rev()
        .find(|tab| tab.host_id == Some(host_id))
        .map(|tab| &tab.status);

    match status {
        Some(SessionStatus::Connected) | Some(SessionStatus::RunningCommand) => "Connected",
        Some(SessionStatus::Connecting)
        | Some(SessionStatus::Authenticating)
        | Some(SessionStatus::Reconnecting) => "Connecting",
        Some(SessionStatus::Failed { .. }) => "Failed",
        Some(SessionStatus::Disconnected) => "Disconnected",
        Some(SessionStatus::Created) => "Created",
        None => "Saved",
    }
}
