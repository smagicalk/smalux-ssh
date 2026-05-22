//! 本地终端输入回显。

use crate::model::{SessionId, SessionKind};
use crate::terminal::TerminalManager;

pub(super) fn echo_local_terminal_input(
    terminal: &mut TerminalManager,
    session_id: SessionId,
    kind: &SessionKind,
    trimmed_input: &str,
    input: &str,
) {
    if !matches!(kind, SessionKind::LocalShell) || trimmed_input.is_empty() {
        return;
    }

    terminal.append_local_echo(
        session_id,
        crate::backend::LocalShellProfile::default_for_platform().prompt,
        input,
    );
}
