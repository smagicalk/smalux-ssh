//! Tunnel 标签页关闭前置判断和运行态清理。

use crate::backend::BackendCommand;
use crate::core::CoreState;
use crate::model::{SessionId, SessionKind, SessionTab, TunnelStatus};

impl CoreState {
    pub(super) fn tunnel_requires_stop_before_close(
        &self,
        session_id: SessionId,
        rule_name: &str,
    ) -> bool {
        self.sessions.tunnels.iter().any(|tunnel| {
            tunnel.session_id == session_id
                && tunnel.rule_name == rule_name
                && !tunnel.status.is_terminal()
        })
    }

    pub(super) fn can_cancel_pending_tunnel_launch(
        &self,
        session_id: SessionId,
        rule_name: &str,
    ) -> bool {
        let runtime_is_starting = self.sessions.tunnels.iter().any(|tunnel| {
            tunnel.session_id == session_id
                && tunnel.rule_name == rule_name
                && tunnel.status == TunnelStatus::Starting
        });

        runtime_is_starting
            && self
                .backend_commands
                .iter()
                .any(|command| is_tunnel_launch_command(command, session_id, rule_name))
    }

    pub(super) fn remove_tunnel_runtime_after_tab_close(&mut self, tab: &SessionTab) -> bool {
        let SessionKind::Tunnel { rule_name } = &tab.kind else {
            return false;
        };

        let before = self.sessions.tunnels.len();
        self.sessions
            .tunnels
            .retain(|tunnel| tunnel.session_id != tab.id || tunnel.rule_name != *rule_name);
        before != self.sessions.tunnels.len()
    }
}

fn is_tunnel_launch_command(
    command: &BackendCommand,
    session_id: SessionId,
    rule_name: &str,
) -> bool {
    match command {
        BackendCommand::Connect {
            session_id: command_session_id,
            ..
        } => *command_session_id == session_id,
        BackendCommand::StartTunnel {
            session_id: command_session_id,
            request,
        } => *command_session_id == session_id && request.rule.name == rule_name,
        _ => false,
    }
}
