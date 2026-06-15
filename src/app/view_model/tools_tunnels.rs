//! 隧道工具页展示模型。

use crate::app::state::AsDesktopStateView;
use crate::model::TunnelStatus;

use super::i18n::{locale_for_state, tr};
use super::tools_types::ToolItemViewModel;

pub(in crate::app::view_model) fn tunnel_items(
    state: impl AsDesktopStateView,
) -> Vec<ToolItemViewModel> {
    let state = state.as_desktop_state_view();
    let locale = locale_for_state(state);
    let saved = state
        .storage
        .tunnel_rules
        .iter()
        .map(|rule| ToolItemViewModel {
            title: rule.name.clone(),
            subtitle: rule.display_endpoint(),
            meta: if rule.auto_start {
                tr(locale, "tool.tunnel_auto")
            } else {
                tr(locale, "tool.tunnel_saved")
            }
            .to_owned(),
        });
    let runtime = state
        .sessions
        .tunnels
        .iter()
        .map(|tunnel| ToolItemViewModel {
            title: tunnel.rule_name.clone(),
            subtitle: tunnel
                .last_error
                .clone()
                .unwrap_or_else(|| tr(locale, "tool.tunnel_runtime").to_owned()),
            meta: tunnel_status_label(&tunnel.status, locale).to_owned(),
        });

    saved.chain(runtime).collect()
}

fn tunnel_status_label(status: &TunnelStatus, locale: super::i18n::Locale) -> &'static str {
    match status {
        TunnelStatus::Stopped => tr(locale, "tool.tunnel_status_stopped"),
        TunnelStatus::Starting => tr(locale, "tool.tunnel_status_starting"),
        TunnelStatus::Running => tr(locale, "tool.tunnel_status_running"),
        TunnelStatus::Stopping => tr(locale, "tool.tunnel_status_stopping"),
        TunnelStatus::Failed => tr(locale, "tool.tunnel_status_failed"),
    }
}
