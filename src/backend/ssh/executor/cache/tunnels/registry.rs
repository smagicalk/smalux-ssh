use std::collections::HashMap;

use crate::model::SessionId;

use super::traits::{RuleNamedTunnel, StoppableTunnel, TunnelOwner};

pub(in crate::backend::ssh::executor) fn remove_tunnel_for_session_rule<TTunnel>(
    tunnels: &mut HashMap<String, TTunnel>,
    session_id: SessionId,
    rule_name: &str,
) -> Option<TTunnel>
where
    TTunnel: TunnelOwner,
{
    if !tunnels
        .get(rule_name)
        .is_some_and(|tunnel| tunnel.session_id() == session_id)
    {
        return None;
    }

    tunnels.remove(rule_name)
}

pub(in crate::backend::ssh::executor) fn replace_tunnel_stopping_previous<TTunnel>(
    tunnels: &mut HashMap<String, TTunnel>,
    tunnel: TTunnel,
) where
    TTunnel: RuleNamedTunnel + StoppableTunnel,
{
    if let Some(previous) = tunnels.insert(tunnel.rule_name().to_owned(), tunnel) {
        previous.stop();
    }
}

pub(in crate::backend::ssh::executor) fn take_tunnels_for_session<TTunnel>(
    tunnels: &mut HashMap<String, TTunnel>,
    session_id: SessionId,
) -> Vec<TTunnel>
where
    TTunnel: TunnelOwner,
{
    let rule_names = tunnels
        .iter()
        .filter_map(|(rule_name, tunnel)| {
            (tunnel.session_id() == session_id).then(|| rule_name.clone())
        })
        .collect::<Vec<_>>();

    rule_names
        .into_iter()
        .filter_map(|rule_name| tunnels.remove(&rule_name))
        .collect()
}
