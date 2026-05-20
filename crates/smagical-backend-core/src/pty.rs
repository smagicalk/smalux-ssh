//! 交互式 PTY 和远程命令请求模型。

use smagical_terminal::TerminalSize;

/// 后端打开交互式 shell 时需要的 PTY 参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyRequest {
    pub term: String,
    pub size: TerminalSize,
    pub environment: Vec<(String, String)>,
}

impl PtyRequest {
    /// 使用终端状态中的尺寸创建默认 xterm 兼容 PTY 请求。
    pub fn xterm(size: TerminalSize) -> Self {
        Self {
            term: "xterm-256color".to_owned(),
            size,
            environment: Vec::new(),
        }
    }
}

/// 后端执行一次性远程命令的请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCommandRequest {
    pub command: String,
    pub pty: Option<PtyRequest>,
}

impl RemoteCommandRequest {
    /// 创建不申请 PTY 的远程命令请求，适合脚本和批处理输出。
    pub fn exec(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            pty: None,
        }
    }

    /// 创建带 PTY 的远程命令请求，适合需要终端能力的交互命令。
    pub fn with_pty(command: impl Into<String>, pty: PtyRequest) -> Self {
        Self {
            command: command.into(),
            pty: Some(pty),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_request_uses_xterm_defaults() {
        let pty = PtyRequest::xterm(TerminalSize::new(0, 24));

        assert_eq!(pty.term, "xterm-256color");
        assert_eq!(pty.size.columns, 1);
        assert_eq!(pty.size.rows, 24);
        assert!(pty.environment.is_empty());
    }

    #[test]
    fn remote_command_can_request_pty() {
        let request =
            RemoteCommandRequest::with_pty("top", PtyRequest::xterm(TerminalSize::new(120, 32)));

        assert_eq!(request.command, "top");
        assert!(request.pty.is_some());
        assert!(RemoteCommandRequest::exec("uptime").pty.is_none());
    }
}
