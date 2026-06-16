use super::*;
use crate::app::state::{DesktopAppState, DesktopStateView};
use crate::core::CoreState;
use crate::model::{
    DEFAULT_LOCAL_TERMINAL_TITLE, HostId, LOCAL_TERMINAL_SESSION_ID, SessionId, SessionStatus,
    UiState,
};
use crate::terminal::TerminalTabState;
use uuid::Uuid;

fn desktop_state() -> DesktopAppState {
    let core = CoreState::default();
    let ui = UiState::from_visual(&core.config.theme, &core.config.background);
    DesktopAppState { core, ui }
}

fn view(state: &DesktopAppState) -> DesktopStateView<'_> {
    state.as_desktop_state_view()
}

#[test]
fn active_terminal_projects_buffer_lines() {
    let mut state = desktop_state();
    let session_id = SessionId(Uuid::new_v4());
    let host_id = HostId(Uuid::new_v4());
    state
        .core
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .core
        .terminal
        .open_tab(TerminalTabState::new(session_id, "production"));
    state.core.terminal.append_output(session_id, "connected");

    let terminal = active_terminal(view(&state));

    assert_eq!(terminal.output_lines, vec!["connected"]);
    assert!(terminal.can_send_input);
    assert!(!terminal.can_reconnect_shell);
    assert_eq!(terminal.prompt, "$");
}

#[test]
fn active_terminal_uses_local_shell_prompt_for_local_tabs() {
    let mut state = desktop_state();
    state
        .core
        .sessions
        .open_local_shell_tab(LOCAL_TERMINAL_SESSION_ID, DEFAULT_LOCAL_TERMINAL_TITLE);

    let terminal = active_terminal(view(&state));

    assert_eq!(
        terminal.prompt,
        crate::backend::LocalShellProfile::default_for_platform().prompt
    );
}

#[test]
fn active_terminal_keeps_shell_when_sftp_tab_is_active() {
    let mut state = desktop_state();
    let host_id = HostId(Uuid::new_v4());
    let shell_id = SessionId(Uuid::new_v4());
    let sftp_id = SessionId(Uuid::new_v4());
    state
        .core
        .sessions
        .open_shell_tab(shell_id, host_id, "production");
    state
        .core
        .sessions
        .set_status(shell_id, SessionStatus::Connected);
    state
        .core
        .terminal
        .open_tab(TerminalTabState::new(shell_id, "production"));
    state
        .core
        .sessions
        .open_sftp_tab(sftp_id, host_id, "/var/log");

    let terminal = active_terminal(view(&state));

    assert_eq!(terminal.session_id, shell_id.0.to_string());
    assert_eq!(terminal.host_id, host_id.0.to_string());
    assert_eq!(terminal.title, "production");
    assert!(terminal.can_send_input);
}

#[test]
fn active_terminal_disables_remote_shell_input_when_terminal() {
    let mut state = desktop_state();
    let session_id = SessionId(Uuid::new_v4());
    let host_id = HostId(Uuid::new_v4());
    state
        .core
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    state
        .core
        .terminal
        .open_tab(TerminalTabState::new(session_id, "production"));

    let terminal = active_terminal(view(&state));

    assert_eq!(terminal.session_id, session_id.0.to_string());
    assert!(!terminal.can_send_input);
    assert!(terminal.can_reconnect_shell);
}

#[test]
fn active_terminal_reconnect_action_is_disabled_for_remote_command_tabs() {
    let mut state = desktop_state();
    let session_id = SessionId(Uuid::new_v4());
    let host_id = HostId(Uuid::new_v4());
    state
        .core
        .sessions
        .open_remote_command_tab(session_id, host_id, "uptime", None);
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    state
        .core
        .terminal
        .open_tab(TerminalTabState::new(session_id, "uptime"));

    let terminal = active_terminal(view(&state));

    assert_eq!(terminal.session_id, session_id.0.to_string());
    assert!(!terminal.can_send_input);
    assert!(!terminal.can_reconnect_shell);
}
