//! 会话标签和终端展示模型。

use crate::model::AppState;

use super::i18n::{english_locale, locale_for_state};
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
    pub status_key: &'static str,
    pub status: &'static str,
    pub active: bool,
}

pub(super) fn tabs(state: &AppState) -> Vec<SessionTabViewModel> {
    let locale = locale_for_state(state);

    state
        .sessions
        .tabs
        .iter()
        .map(|tab| SessionTabViewModel {
            id: tab.id.0.to_string(),
            title: tab.title.clone(),
            kind: session_kind_label(&tab.kind, locale),
            status_key: session_status_label(&tab.status, english_locale()),
            status: session_status_label(&tab.status, locale),
            active: state.sessions.active_tab == Some(tab.id),
        })
        .collect()
}
