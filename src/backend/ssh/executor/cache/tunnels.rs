//! SSH executor 隧道缓存操作。

#[path = "tunnels/cleanup.rs"]
mod cleanup;
#[path = "tunnels/registry.rs"]
mod registry;
#[path = "tunnels/traits.rs"]
mod traits;

pub(in crate::backend::ssh::executor) use cleanup::stop_detached_tunnels;
pub(in crate::backend::ssh::executor) use registry::{
    remove_tunnel_for_session_rule, replace_tunnel_stopping_previous, take_tunnels_for_session,
};
pub(in crate::backend::ssh::executor) use traits::TunnelOwner;
#[cfg(test)]
pub(in crate::backend::ssh::executor) use traits::{RuleNamedTunnel, StoppableTunnel};

#[cfg(test)]
#[path = "tunnels_tests.rs"]
mod tests;
