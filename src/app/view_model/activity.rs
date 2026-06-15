//! 右侧活动栏展示模型。

use crate::app::state::AsDesktopStateView;

use super::common::background_summary;
use super::i18n::{locale_for_state, tr};
use super::labels::theme_label;

/// 活动侧栏指标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct ActivityViewModel {
    pub label: &'static str,
    pub value: String,
}

pub(super) fn activity(state: impl AsDesktopStateView) -> Vec<ActivityViewModel> {
    let state = state.as_desktop_state_view();
    let locale = locale_for_state(state);

    vec![
        metric(
            tr(locale, "activity.metric.hosts"),
            state.storage.host_count(),
        ),
        metric(
            tr(locale, "activity.metric.tabs"),
            state.sessions.tab_count(),
        ),
        metric(
            tr(locale, "activity.metric.active"),
            state.sessions.active_count(),
        ),
        metric(
            tr(locale, "activity.metric.sftp"),
            state.sessions.sftp_browser_count(),
        ),
        metric(
            tr(locale, "activity.metric.tunnels"),
            state.sessions.tunnel_runtime_count(),
        ),
        ActivityViewModel {
            label: tr(locale, "activity.metric.language"),
            value: state.ui.workspace.language_label().to_owned(),
        },
        ActivityViewModel {
            label: tr(locale, "activity.metric.theme"),
            value: theme_label(state.ui.workspace.theme, locale).to_owned(),
        },
        ActivityViewModel {
            label: tr(locale, "activity.metric.background"),
            value: background_summary(state),
        },
    ]
}

fn metric(label: &'static str, value: usize) -> ActivityViewModel {
    ActivityViewModel {
        label,
        value: value.to_string(),
    }
}
