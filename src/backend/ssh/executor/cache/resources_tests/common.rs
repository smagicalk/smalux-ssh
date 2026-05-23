use std::collections::HashMap;

use uuid::Uuid;

use crate::backend::ssh::executor::cache::tunnels::{
    RuleNamedTunnel, StoppableTunnel, TunnelOwner,
};
use crate::model::SessionId;

pub(super) fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestTunnel {
    pub(super) session_id: SessionId,
    pub(super) rule_name: String,
}

impl TunnelOwner for TestTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id
    }
}

impl RuleNamedTunnel for TestTunnel {
    fn rule_name(&self) -> &str {
        &self.rule_name
    }
}

impl StoppableTunnel for TestTunnel {
    fn stop(&self) {}
}

pub(super) fn shells<T>(items: impl IntoIterator<Item = (SessionId, T)>) -> HashMap<SessionId, T> {
    HashMap::from_iter(items)
}

pub(super) fn sftps<T>(items: impl IntoIterator<Item = (SessionId, T)>) -> HashMap<SessionId, T> {
    HashMap::from_iter(items)
}

pub(super) fn connections<T>(
    items: impl IntoIterator<Item = (SessionId, T)>,
) -> HashMap<SessionId, T> {
    HashMap::from_iter(items)
}

pub(super) fn tunnels<T>(items: impl IntoIterator<Item = (String, T)>) -> HashMap<String, T> {
    HashMap::from_iter(items)
}
