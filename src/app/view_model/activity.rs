//! 右侧活动栏展示模型。

use crate::model::AppState;

use super::common::background_summary;
use super::labels::theme_label;

/// 活动侧栏指标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct ActivityViewModel {
    pub label: &'static str,
    pub value: String,
}

pub(super) fn activity(state: &AppState) -> Vec<ActivityViewModel> {
    vec![
        metric("Hosts", state.storage.host_count()),
        metric("Tabs", state.sessions.tab_count()),
        metric("Active", state.sessions.active_count()),
        metric("SFTP", state.sessions.sftp_browser_count()),
        metric("Tunnels", state.sessions.tunnel_runtime_count()),
        ActivityViewModel {
            label: "Language",
            value: state.ui.workspace.language_label().to_owned(),
        },
        ActivityViewModel {
            label: "Theme",
            value: theme_label(state.ui.workspace.theme).to_owned(),
        },
        ActivityViewModel {
            label: "Background",
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
