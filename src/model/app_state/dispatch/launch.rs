//! 会话启动和隧道消息路由。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn dispatch_launch_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::OpenShell { host_id } => self.open_shell(host_id),
            Message::OpenRecentConnection { host_id } => self.open_recent_connection(host_id),
            Message::ReconnectShell { session_id } => self.reconnect_shell(session_id),
            Message::OpenSftp {
                host_id,
                initial_dir,
            } => self.open_sftp(host_id, initial_dir),
            Message::RunRemoteCommand {
                host_id,
                command,
                request_pty,
            } => self.run_remote_command(host_id, command, request_pty),
            Message::StartTunnel { host_id, rule } => self.start_tunnel(host_id, rule),
            Message::StopTunnel {
                session_id,
                rule_name,
            } => self.stop_tunnel(session_id, rule_name),
            _ => unreachable!("非启动消息不应进入启动路由"),
        }
    }
}
