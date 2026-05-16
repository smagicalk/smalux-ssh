//! 本地命令 fallback 执行器。
//!
//! 真实 PTY 在某些 Windows ConPTY 环境下可能无法稳定返回输出。
//! 本模块只负责进程执行；shell 差异由 `LocalShellProfile` 描述，
//! VT/ANSI 字节流仍交给终端核心解析。

use std::process::Command;

use crate::model::SessionId;
use crate::terminal::TerminalStreamDecoder;

use super::{BackendEvent, LocalShellProfile};

/// 本地命令 fallback 执行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCommandFallbackResult {
    pub events: Vec<BackendEvent>,
    pub exit_code: Option<i32>,
}

/// 执行一条本地命令并转换为后端事件。
pub struct LocalCommandFallback;

impl LocalCommandFallback {
    pub fn execute(session_id: SessionId, input: &str) -> LocalCommandFallbackResult {
        Self::execute_with_profile(
            session_id,
            input,
            &LocalShellProfile::default_for_platform(),
        )
    }

    pub fn execute_with_profile(
        session_id: SessionId,
        input: &str,
        shell: &LocalShellProfile,
    ) -> LocalCommandFallbackResult {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return LocalCommandFallbackResult {
                events: Vec::new(),
                exit_code: None,
            };
        }

        if let Some(sequence) = shell.control_sequence(trimmed) {
            return LocalCommandFallbackResult {
                events: decode_terminal_bytes(session_id, sequence),
                exit_code: Some(0),
            };
        }

        let command = input.trim_end_matches(['\r', '\n']);
        let output = execute_local_command(shell, command);
        let mut events = vec![BackendEvent::Output {
            session_id,
            line: format!("{} {command}", shell.prompt),
        }];
        events.extend(decode_terminal_bytes(session_id, &output.stdout));
        events.extend(decode_terminal_bytes(session_id, &output.stderr));

        LocalCommandFallbackResult {
            events,
            exit_code: output.exit_code,
        }
    }
}

struct LocalCommandOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: Option<i32>,
}

fn execute_local_command(shell: &LocalShellProfile, command: &str) -> LocalCommandOutput {
    let fallback = shell.fallback_command(command);
    let output = Command::new(&fallback.program)
        .args(&fallback.args)
        .output();

    match output {
        Ok(output) => LocalCommandOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.status.code(),
        },
        Err(error) => LocalCommandOutput {
            stdout: Vec::new(),
            stderr: format!("local command failed: {error}").into_bytes(),
            exit_code: None,
        },
    }
}

fn decode_terminal_bytes(session_id: SessionId, bytes: &[u8]) -> Vec<BackendEvent> {
    let mut decoder = TerminalStreamDecoder::new();
    let mut events = decoder.feed(bytes);
    events.extend(decoder.finish());
    events
        .into_iter()
        .map(|event| super::local_pty::terminal_event_to_backend(session_id, event))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::LocalShellKind;
    use crate::model::LOCAL_TERMINAL_SESSION_ID;

    #[test]
    fn local_command_fallback_executes_visible_output() {
        let command = if cfg!(windows) {
            "Write-Output 'smagicalssh-fallback'"
        } else {
            "printf 'smagicalssh-fallback\\n'"
        };

        let result = LocalCommandFallback::execute(LOCAL_TERMINAL_SESSION_ID, command);

        assert!(result.events.iter().any(|event| matches!(
            event,
            BackendEvent::Output { line, .. } if line.contains("smagicalssh-fallback")
        )));
    }

    #[test]
    fn local_command_fallback_maps_clear_alias_through_terminal_core() {
        let result = LocalCommandFallback::execute(LOCAL_TERMINAL_SESSION_ID, "Clear-Host");

        assert_eq!(
            result.events,
            vec![BackendEvent::ClearTerminal {
                session_id: LOCAL_TERMINAL_SESSION_ID,
            }]
        );
    }

    #[test]
    fn local_command_fallback_uses_shell_specific_clear_aliases() {
        let cmd = LocalShellProfile {
            kind: LocalShellKind::Cmd,
            program: "cmd.exe".to_owned(),
            interactive_args: Vec::new(),
            prompt: ">",
        };

        let result =
            LocalCommandFallback::execute_with_profile(LOCAL_TERMINAL_SESSION_ID, "cls", &cmd);

        assert_eq!(
            result.events,
            vec![BackendEvent::ClearTerminal {
                session_id: LOCAL_TERMINAL_SESSION_ID,
            }]
        );
    }

    #[test]
    fn local_command_fallback_preserves_non_control_input_spacing() {
        let posix = LocalShellProfile {
            kind: LocalShellKind::Posix,
            program: "sh".to_owned(),
            interactive_args: Vec::new(),
            prompt: "$",
        };

        let result = LocalCommandFallback::execute_with_profile(
            LOCAL_TERMINAL_SESSION_ID,
            " printf 'spaced\\n' ",
            &posix,
        );

        assert!(result.events.iter().any(|event| matches!(
            event,
            BackendEvent::Output { line, .. } if line == "$  printf 'spaced\\n' "
        )));
    }
}
