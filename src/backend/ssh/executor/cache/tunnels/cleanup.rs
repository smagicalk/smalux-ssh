use crate::model::SessionId;

use super::traits::{RuleNamedTunnel, StoppableTunnel};

pub(in crate::backend::ssh::executor) fn stop_detached_tunnels<TTunnel>(
    session_id: SessionId,
    tunnels: Vec<TTunnel>,
    operation: &'static str,
) where
    TTunnel: RuleNamedTunnel + StoppableTunnel,
{
    for tunnel in tunnels {
        let rule_name = tunnel.rule_name().to_owned();
        tunnel.stop();
        tracing::warn!(
            session_id = %session_id.0,
            operation,
            rule_name,
            "stopped detached SSH tunnel"
        );
    }
}
