use super::*;
use crate::model::{
    DEFAULT_LOCAL_TERMINAL_TITLE, HostId, LOCAL_TERMINAL_SESSION_ID, SessionId, SessionStatus,
};
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
        .sessions
        .set_status(session_id, SessionStatus::Connected);
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
        .sessions
        .set_status(shell_id, SessionStatus::Connected);
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

#[test]
fn active_terminal_disables_remote_shell_input_when_terminal() {
    let mut state = AppState::default();
    let session_id = SessionId(Uuid::new_v4());
    let host_id = HostId(Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    state
        .terminal
        .open_tab(TerminalTabState::new(session_id, "production"));

    let terminal = active_terminal(&state);

    assert_eq!(terminal.session_id, session_id.0.to_string());
    assert!(!terminal.can_send_input);
}
