//! 会话标签和终端展示模型。

use crate::model::{
    AppState, DEFAULT_LOCAL_TERMINAL_TITLE, LOCAL_TERMINAL_SESSION_ID, SessionKind,
};

use super::labels::{session_kind_label, session_status_label};

/// 会话标签展示行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SessionTabViewModel {
    pub id: String,
    pub title: String,
    pub kind: &'static str,
    pub status: &'static str,
    pub active: bool,
}

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

pub(in crate::app) fn active_terminal(state: &AppState) -> TerminalViewModel {
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
            kind: "Local",
            status: "Ready",
            output_lines,
            input: state
                .ui
                .terminal_input_for(LOCAL_TERMINAL_SESSION_ID)
                .to_owned(),
            prompt: local_terminal_prompt(),
            can_send_input: true,
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
        kind: session_kind_label(&tab.kind),
        status: session_status_label(&tab.status),
        output_lines,
        input: state.ui.terminal_input_for(tab.id).to_owned(),
        prompt: terminal_prompt_for_kind(&tab.kind),
        can_send_input: matches!(tab.kind, SessionKind::LocalShell | SessionKind::Shell),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HostId, SessionId};
    use crate::terminal::TerminalTabState;
    use uuid::Uuid;

    #[test]
    fn active_terminal_projects_buffer_lines() {
        let mut state = AppState::default();
        let session_id = SessionId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());
        state
            .sessions
            .open_shell_tab(session_id, host_id, "production");
        state
            .terminal
            .open_tab(TerminalTabState::new(session_id, "production"));
        state.terminal.append_output(session_id, "connected");

        let terminal = active_terminal(&state);

        assert_eq!(terminal.output_lines, vec!["connected"]);
        assert!(terminal.can_send_input);
        assert_eq!(terminal.prompt, "$");
    }

    #[test]
    fn active_terminal_uses_local_shell_prompt_for_local_tabs() {
        let mut state = AppState::default();
        state
            .sessions
            .open_local_shell_tab(LOCAL_TERMINAL_SESSION_ID, DEFAULT_LOCAL_TERMINAL_TITLE);

        let terminal = active_terminal(&state);

        assert_eq!(
            terminal.prompt,
            crate::backend::LocalShellProfile::default_for_platform().prompt
        );
    }

    #[test]
    fn active_terminal_keeps_shell_when_sftp_tab_is_active() {
        let mut state = AppState::default();
        let host_id = HostId(Uuid::new_v4());
        let shell_id = SessionId(Uuid::new_v4());
        let sftp_id = SessionId(Uuid::new_v4());
        state
            .sessions
            .open_shell_tab(shell_id, host_id, "production");
        state
            .terminal
            .open_tab(TerminalTabState::new(shell_id, "production"));
        state.sessions.open_sftp_tab(sftp_id, host_id, "/var/log");

        let terminal = active_terminal(&state);

        assert_eq!(terminal.session_id, shell_id.0.to_string());
        assert_eq!(terminal.host_id, host_id.0.to_string());
        assert_eq!(terminal.title, "production");
        assert!(terminal.can_send_input);
    }
}
