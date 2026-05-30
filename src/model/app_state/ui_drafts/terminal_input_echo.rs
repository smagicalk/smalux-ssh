//! 本地终端输入回显。
//!
//! 真实 PTY 启动前，本地终端输入可能需要立即给用户反馈。这个模块只处理本地 shell 的
//! UI 回显，不影响远程 shell；远程 shell 的输出应来自后端 drain。

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

    // 使用平台默认 profile 的 prompt，让本地回显和实际本地 shell 风格保持接近。
    terminal.append_local_echo(
        session_id,
        crate::backend::LocalShellProfile::default_for_platform().prompt,
        input,
    );
}
