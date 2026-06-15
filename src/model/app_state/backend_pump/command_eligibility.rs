//! 后端命令是否仍可执行的状态判定。
//!
//! 后端命令从入队到真正执行之间存在时间差：用户可能关闭标签页、删除主机、取消传输，
//! 或会话已经失败。因此每条命令在执行前都要重新检查当前状态。这个模块只做“是否还能
//! 执行”的判定，不做状态修改；真正的清理交给 `stale_commands`。

use crate::backend::{BackendCommand, SftpRequest};
use crate::core::CoreState;

impl CoreState {
    pub(super) fn can_execute_backend_command(&self, command: &BackendCommand) -> bool {
        match command {
            // 连接类命令要求会话仍存在，并且目标主机没有在命令入队后被删除。
            BackendCommand::Connect { session_id, target } => self
                .sessions
                .can_execute_connect_command(*session_id, target.host_id),
            // 交互式输入和 drain 输出只允许作用于仍然处于交互态的 shell。
            BackendCommand::SendShellInput { session_id, .. } => {
                self.sessions.can_send_interactive_shell_input(*session_id)
            }
            BackendCommand::DrainSessionOutput { session_id } => {
                self.sessions.can_drain_interactive_shell(*session_id)
            }
            // 打开 shell 是连接完成后的第二阶段，必须确认会话仍在等待 shell。
            BackendCommand::OpenShell { session_id, .. } => {
                self.sessions.can_execute_open_shell_command(*session_id)
            }
            // 本地 shell 依赖本地 PTY 启动状态，仍要确认标签页没有被关闭。
            BackendCommand::OpenLocalShell { session_id, .. } => self
                .sessions
                .can_execute_open_local_shell_command(*session_id),
            // 远程命令依赖已连接的非交互命令会话。
            BackendCommand::RunCommand { session_id, .. } => {
                self.sessions.can_execute_remote_command(*session_id)
            }
            // SFTP 浏览命令只影响目录树 loading 状态。
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::ListDir { .. },
            } => self.sessions.can_execute_sftp_browser_command(*session_id),
            // SFTP 写操作会改远端目录，但不一定有传输进度条。
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::RemoveFile { .. } | SftpRequest::CreateDir { .. },
            } => self.sessions.can_execute_sftp_browser_command(*session_id),
            // 上传/下载有独立 transfer 状态，判定规则更严格。
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::Upload { .. } | SftpRequest::Download { .. },
            } => self.sessions.can_execute_sftp_transfer_command(*session_id),
            // 隧道按规则名去重，避免重复启动或停止同一个规则。
            BackendCommand::StartTunnel {
                session_id,
                request,
            } => self
                .sessions
                .can_execute_tunnel_start_command(*session_id, &request.rule.name),
            BackendCommand::StopTunnel {
                session_id,
                request,
            } => self
                .sessions
                .can_execute_tunnel_stop_command(*session_id, &request.rule_name),
            // 其他命令当前不依赖易失 UI 状态，默认允许执行。
            _ => true,
        }
    }
}
