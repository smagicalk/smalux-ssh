//! 会话标签和终端展示模型。

use crate::model::AppState;

use super::labels::{session_kind_label, session_status_label};

mod terminal;
#[cfg(test)]
mod tests;

pub(in crate::app) use terminal::{TerminalViewModel, active_terminal};

/// 会话标签展示行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SessionTabViewModel {
    pub id: String,
    pub title: String,
    pub kind: &'static str,
    pub status: &'static str,
    pub active: bool,
}

pub(super) fn tabs(state: &AppState) -> Vec<SessionTabViewModel> {
    state
        .sessions
        .tabs
        .iter()
        .map(|tab| SessionTabViewModel {
            id: tab.id.0.to_string(),
            title: tab.title.clone(),
            kind: session_kind_label(&tab.kind),
            status: session_status_label(&tab.status),
            active: state.sessions.active_tab == Some(tab.id),
        })
        .collect()
}
