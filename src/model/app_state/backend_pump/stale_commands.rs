//! 后端泵中过期命令的状态收尾。
//!
//! “过期命令”指命令曾经合法入队，但执行前它依赖的会话或 UI 状态已经不存在。这里不把
//! 它当成后端错误，因为后端还没有真正执行；状态层只需要把用户可见的 loading、失败
//! 提示和传输进度收尾，保证界面不会一直停在“连接中/传输中”。

use crate::backend::BackendCommand;

use super::super::{AppState, AppUpdateOutcome};

#[path = "stale_connect.rs"]
mod stale_connect;
#[path = "stale_local_shell.rs"]
mod stale_local_shell;
#[path = "stale_remote_command.rs"]
mod stale_remote_command;
#[path = "stale_sftp.rs"]
mod stale_sftp;

impl AppState {
    pub(super) fn skip_stale_backend_command(
        &mut self,
        command: &BackendCommand,
    ) -> AppUpdateOutcome {
        match command {
            // 连接过期通常意味着标签页已关闭，需要同时清掉同会话的后续命令。
            BackendCommand::Connect { session_id, .. } => {
                self.skip_stale_connect_command(*session_id)
            }
            // 本地 shell 启动失败要给终端标签页一个明确失败状态。
            BackendCommand::OpenLocalShell { session_id, .. } => {
                self.skip_stale_local_shell_command(*session_id)
            }
            // SFTP 不同请求对应不同 UI 状态：浏览 loading、写操作错误、传输进度。
            BackendCommand::Sftp {
                session_id,
                request,
            } => self.skip_stale_sftp_command(*session_id, request, command),
            // 远程命令过期只需要结束历史记录的运行态。
            BackendCommand::RunCommand { session_id, .. } => {
                self.skip_stale_remote_command(*session_id)
            }
            // 无可见副作用的命令直接丢弃。
            _ => AppUpdateOutcome::default(),
        }
    }
}
