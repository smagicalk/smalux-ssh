use crate::backend::ssh::RemoteTunnel;
use crate::model::SessionId;

pub(in crate::backend::ssh::executor) trait TunnelOwner {
    fn session_id(&self) -> SessionId;
}

pub(in crate::backend::ssh::executor) trait RuleNamedTunnel {
    fn rule_name(&self) -> &str;
}

pub(in crate::backend::ssh::executor) trait StoppableTunnel {
    fn stop(&self);
}

impl TunnelOwner for RemoteTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id()
    }
}

impl RuleNamedTunnel for RemoteTunnel {
    fn rule_name(&self) -> &str {
        RemoteTunnel::rule_name(self)
    }
}

impl StoppableTunnel for RemoteTunnel {
    fn stop(&self) {
        RemoteTunnel::stop(self);
    }
}
