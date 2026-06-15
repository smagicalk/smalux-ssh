//! 会话启动和隧道消息路由。
//!
//! 这里处理会创建后端命令或会话运行态的动作。UI 只表达“打开 shell”、
//! “运行命令”或“启动隧道”的意图，具体校验、历史记录和后端命令排队都在
//! launch 子模块中完成。

use crate::core::CoreState;

use super::super::{AppState, AppUpdateOutcome, Message};

impl CoreState {
    /// 分发启动类核心消息。
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

impl AppState {
    /// 分发启动类消息。
    ///
    /// 这些消息大多会同时修改 `sessions`、`terminal`、`storage` 和
    /// `backend_commands`，因此集中路由能避免 UI 直接拼装跨模块流程。
    pub(super) fn dispatch_launch_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::OpenShell { host_id } => {
                let outcome = self.core.open_shell(host_id);
                if outcome.changed() {
                    self.ui.workspace.active_page = crate::model::WorkspacePage::Terminal;
                }
                outcome
            }
            Message::OpenRecentConnection { host_id } => {
                let outcome = self.core.open_recent_connection(host_id);
                if outcome.changed() {
                    self.ui.workspace.active_page = crate::model::WorkspacePage::Terminal;
                }
                outcome
            }
            Message::ReconnectShell { session_id } => {
                let outcome = self.core.reconnect_shell(session_id);
                if outcome.changed() {
                    self.ui.workspace.active_page = crate::model::WorkspacePage::Terminal;
                }
                outcome
            }
            Message::OpenSftp {
                host_id,
                initial_dir,
            } => {
                let outcome = self.core.open_sftp(host_id, initial_dir);
                if outcome.changed() {
                    self.ui.workspace.active_page = crate::model::WorkspacePage::Sftp;
                }
                outcome
            }
            Message::RunRemoteCommand {
                host_id,
                command,
                request_pty,
            } => {
                let outcome = self.core.run_remote_command(host_id, command, request_pty);
                if outcome.changed() {
                    self.ui.workspace.active_page = crate::model::WorkspacePage::Terminal;
                }
                outcome
            }
            Message::StartTunnel { host_id, rule } => {
                let outcome = self.core.start_tunnel(host_id, rule);
                if outcome.changed() {
                    self.ui.workspace.active_page = crate::model::WorkspacePage::Tunnels;
                }
                outcome
            }
            Message::StopTunnel {
                session_id,
                rule_name,
            } => self.core.stop_tunnel(session_id, rule_name),
            _ => unreachable!("非启动消息不应进入启动路由"),
        }
    }
}
