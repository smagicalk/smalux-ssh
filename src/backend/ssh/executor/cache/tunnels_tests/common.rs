use uuid::Uuid;

use crate::backend::ssh::executor::cache::tunnels::{
    RuleNamedTunnel, StoppableTunnel, TunnelOwner,
};
use crate::model::SessionId;

pub(super) fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OwnedTunnel {
    pub(super) session_id: SessionId,
}

impl TunnelOwner for OwnedTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TestTunnel {
    pub(super) session_id: SessionId,
    pub(super) rule_name: String,
    pub(super) stopped: bool,
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
    fn stop(&self) {
        STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().push(self.rule_name.clone()));
    }
}

thread_local! {
    static STOPPED_TEST_TUNNEL_NAMES: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

pub(super) fn clear_stopped_tunnel_names() {
    STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());
}

pub(super) fn stopped_tunnel_names() -> Vec<String> {
    STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().clone())
}
