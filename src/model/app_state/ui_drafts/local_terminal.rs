//! 本地终端标签页确保逻辑。

use crate::model::SessionId;
use crate::terminal::TerminalTabState;

use super::super::AppState;

pub(in crate::model::app_state) fn ensure_local_terminal_tab(
    state: &mut AppState,
    session_id: SessionId,
) -> bool {
    let had_session = state.sessions.tabs.iter().any(|tab| tab.id == session_id);
    if !had_session {
        state
            .sessions
            .open_local_shell_tab(session_id, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    }

    let had_terminal = state
        .terminal
        .tabs
        .iter()
        .any(|tab| tab.session_id == session_id);
    if !had_terminal {
        state.terminal.open_tab(TerminalTabState::new(
            session_id,
            crate::model::DEFAULT_LOCAL_TERMINAL_TITLE,
        ));
    }

    !had_session || !had_terminal
}
