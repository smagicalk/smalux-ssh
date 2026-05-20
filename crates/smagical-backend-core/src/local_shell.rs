//! 本地 shell 适配。
//!
//! 这里负责选择和描述本地 shell。终端核心只处理 VT/ANSI 字节流，
//! 不关心 PowerShell、cmd、bash 或 zsh 的命令行差异。

use std::borrow::Cow;

const CLEAR_SCREEN_SEQUENCE: &[u8] = b"\x1b[2J\x1b[H";

/// 本地 shell 类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalShellKind {
    PowerShell,
    Cmd,
    Posix,
}

/// 本地 shell 启动和 fallback 执行配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalShellProfile {
    pub kind: LocalShellKind,
    pub program: String,
    pub interactive_args: Vec<String>,
    pub prompt: &'static str,
}

impl LocalShellProfile {
    /// 当前平台默认 shell。
    pub fn default_for_platform() -> Self {
        if cfg!(windows) {
            Self {
                kind: LocalShellKind::PowerShell,
                program: "powershell.exe".to_owned(),
                interactive_args: vec![
                    "-NoLogo".to_owned(),
                    "-NoExit".to_owned(),
                    "-Command".to_owned(),
                    "$OutputEncoding=[System.Text.UTF8Encoding]::new(); [Console]::InputEncoding=[System.Text.Encoding]::UTF8; [Console]::OutputEncoding=[System.Text.Encoding]::UTF8"
                        .to_owned(),
                ],
                prompt: "PS>",
            }
        } else {
            Self {
                kind: LocalShellKind::Posix,
                program: std::env::var("SHELL").unwrap_or_else(|_| "sh".to_owned()),
                interactive_args: Vec::new(),
                prompt: "$",
            }
        }
    }

    /// 把一条用户输入转换为 fallback 进程调用。
    pub fn fallback_command(&self, input: &str) -> LocalShellFallbackCommand {
        match self.kind {
            LocalShellKind::PowerShell => {
                let command = format!(
                    "[Console]::InputEncoding=[System.Text.Encoding]::UTF8; \
                     [Console]::OutputEncoding=[System.Text.Encoding]::UTF8; \
                     $OutputEncoding=[System.Text.UTF8Encoding]::new(); {input}"
                );
                LocalShellFallbackCommand {
                    program: "powershell.exe".to_owned(),
                    args: vec![
                        "-NoLogo".to_owned(),
                        "-NoProfile".to_owned(),
                        "-NonInteractive".to_owned(),
                        "-ExecutionPolicy".to_owned(),
                        "Bypass".to_owned(),
                        "-Command".to_owned(),
                        command,
                    ],
                }
            }
            LocalShellKind::Cmd => LocalShellFallbackCommand {
                program: "cmd.exe".to_owned(),
                args: vec!["/C".to_owned(), input.to_owned()],
            },
            LocalShellKind::Posix => LocalShellFallbackCommand {
                program: self.program.clone(),
                args: vec!["-lc".to_owned(), input.to_owned()],
            },
        }
    }

    /// 把 shell 专属控制命令转换成通用 VT 序列。
    pub fn control_sequence(&self, input: &str) -> Option<&'static [u8]> {
        let command = input.trim().to_ascii_lowercase();
        match self.kind {
            LocalShellKind::PowerShell => match command.as_str() {
                "clear" | "cls" | "clear-host" => Some(CLEAR_SCREEN_SEQUENCE),
                _ => None,
            },
            LocalShellKind::Cmd => match command.as_str() {
                "cls" => Some(CLEAR_SCREEN_SEQUENCE),
                _ => None,
            },
            LocalShellKind::Posix => match command.as_str() {
                "clear" => Some(CLEAR_SCREEN_SEQUENCE),
                _ => None,
            },
        }
    }

    /// 按平台规范化写入 PTY 的换行。
    pub fn normalize_input<'a>(&self, input: &'a str) -> Cow<'a, str> {
        if cfg!(windows) {
            Cow::Owned(input.replace('\n', "\r\n"))
        } else {
            Cow::Borrowed(input)
        }
    }
}

/// 本地 fallback 进程调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalShellFallbackCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shell_profile_is_platform_specific() {
        let profile = LocalShellProfile::default_for_platform();

        if cfg!(windows) {
            assert_eq!(profile.kind, LocalShellKind::PowerShell);
            assert_eq!(profile.program, "powershell.exe");
            assert_eq!(profile.prompt, "PS>");
            assert!(
                profile
                    .interactive_args
                    .iter()
                    .any(|arg| arg.contains("OutputEncoding"))
            );
        } else {
            assert_eq!(profile.kind, LocalShellKind::Posix);
            assert_eq!(profile.prompt, "$");
        }
    }

    #[test]
    fn clear_aliases_are_shell_specific() {
        let powershell = LocalShellProfile {
            kind: LocalShellKind::PowerShell,
            program: "powershell.exe".to_owned(),
            interactive_args: Vec::new(),
            prompt: "PS>",
        };
        let cmd = LocalShellProfile {
            kind: LocalShellKind::Cmd,
            program: "cmd.exe".to_owned(),
            interactive_args: Vec::new(),
            prompt: ">",
        };
        let posix = LocalShellProfile {
            kind: LocalShellKind::Posix,
            program: "sh".to_owned(),
            interactive_args: Vec::new(),
            prompt: "$",
        };

        assert!(powershell.control_sequence("Clear-Host").is_some());
        assert!(powershell.control_sequence("cls").is_some());
        assert!(cmd.control_sequence("cls").is_some());
        assert!(cmd.control_sequence("clear").is_none());
        assert!(posix.control_sequence("clear").is_some());
        assert!(posix.control_sequence("cls").is_none());
    }
}
