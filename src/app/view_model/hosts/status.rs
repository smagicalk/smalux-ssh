//! 主机连接状态展示标签。
//!
//! 主机状态来自最近一个关联该主机的 session。这里同时提供展示文案和稳定 key：文案走
//! i18n，key 给 Slint 样式判断使用，避免 UI 根据中文/英文文本决定颜色。

use crate::app::state::AsDesktopStateView;
use crate::model::{HostId, SessionStatus};

use super::super::i18n::{Locale, tr};

pub(super) fn host_status_label(
    state: impl AsDesktopStateView,
    host_id: HostId,
    locale: Locale,
) -> &'static str {
    let state = state.as_desktop_state_view();
    // 从后往前找，最近打开的会话最能代表主机当前状态。
    let status = state
        .sessions
        .tabs
        .iter()
        .rev()
        .find(|tab| tab.host_id == Some(host_id))
        .map(|tab| &tab.status);

    match status {
        Some(SessionStatus::Connected) | Some(SessionStatus::RunningCommand) => {
            tr(locale, "host.status.connected")
        }
        Some(SessionStatus::Connecting)
        | Some(SessionStatus::Authenticating)
        | Some(SessionStatus::Reconnecting) => tr(locale, "host.status.connecting"),
        Some(SessionStatus::Failed { .. }) => tr(locale, "host.status.failed"),
        Some(SessionStatus::Disconnected) => tr(locale, "host.status.disconnected"),
        Some(SessionStatus::Created) => tr(locale, "host.status.created"),
        None => tr(locale, "host.status.saved"),
    }
}

pub(super) fn host_status_key(state: impl AsDesktopStateView, host_id: HostId) -> &'static str {
    let state = state.as_desktop_state_view();
    // key 必须保持英文稳定值，不能本地化；UI 主题和状态色依赖它。
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
