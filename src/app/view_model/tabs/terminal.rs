//! 当前终端区域展示模型。

use crate::model::{
    AppState, DEFAULT_LOCAL_TERMINAL_TITLE, LOCAL_TERMINAL_SESSION_ID, SessionKind,
};

use super::super::i18n::{locale_for_state, tr};
use super::super::labels::{session_kind_label, session_status_label};

/// 当前终端区域展示状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct TerminalViewModel {
    pub session_id: String,
    pub host_id: String,
    pub title: String,
    pub kind: &'static str,
    pub status: &'static str,
    pub output_lines: Vec<String>,
    pub input: String,
    pub prompt: &'static str,
    pub can_send_input: bool,
    pub can_reconnect_shell: bool,
}

pub(in crate::app) fn active_terminal(state: &AppState) -> TerminalViewModel {
    let locale = locale_for_state(state);
    let active_tab = state
        .terminal
        .active_tab
        .and_then(|active_id| state.sessions.tabs.iter().find(|tab| tab.id == active_id))
        .or_else(|| {
            state.sessions.tabs.iter().rev().find(|tab| {
                matches!(
                    tab.kind,
                    SessionKind::LocalShell
                        | SessionKind::Shell
                        | SessionKind::RemoteCommand { .. }
                )
            })
        });

    let Some(tab) = active_tab else {
        let output_lines = state
            .terminal
            .tabs
            .iter()
            .find(|terminal_tab| terminal_tab.session_id == LOCAL_TERMINAL_SESSION_ID)
            .map(|terminal_tab| terminal_tab.buffer.clone())
            .unwrap_or_default();

        return TerminalViewModel {
            session_id: LOCAL_TERMINAL_SESSION_ID.0.to_string(),
            host_id: String::new(),
            title: DEFAULT_LOCAL_TERMINAL_TITLE.to_owned(),
            kind: tr(locale, "session.kind.local"),
            status: tr(locale, "session.status.ready"),
            output_lines,
            input: state
                .ui
                .terminal_input_for(LOCAL_TERMINAL_SESSION_ID)
                .to_owned(),
            prompt: local_terminal_prompt(),
            can_send_input: true,
            can_reconnect_shell: false,
        };
    };

    let output_lines = state
        .terminal
        .tabs
        .iter()
        .find(|terminal_tab| terminal_tab.session_id == tab.id)
        .map(|terminal_tab| terminal_tab.buffer.clone())
        .unwrap_or_default();

    TerminalViewModel {
        session_id: tab.id.0.to_string(),
        host_id: tab
            .host_id
            .map(|host_id| host_id.0.to_string())
            .unwrap_or_default(),
        title: tab.title.clone(),
        kind: session_kind_label(&tab.kind, locale),
        status: session_status_label(&tab.status, locale),
        output_lines,
        input: state.ui.terminal_input_for(tab.id).to_owned(),
        prompt: terminal_prompt_for_kind(&tab.kind),
        can_send_input: tab.can_accept_terminal_input(),
        can_reconnect_shell: tab.can_reconnect_shell(),
    }
}

fn terminal_prompt_for_kind(kind: &SessionKind) -> &'static str {
    match kind {
        SessionKind::LocalShell => local_terminal_prompt(),
        SessionKind::Shell => "$",
        SessionKind::RemoteCommand { .. } | SessionKind::Sftp | SessionKind::Tunnel { .. } => ">",
    }
}

fn local_terminal_prompt() -> &'static str {
    crate::backend::LocalShellProfile::default_for_platform().prompt
}
