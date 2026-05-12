use crate::model::{Host, HostGroup, TunnelRule};

#[derive(Debug, Clone, Default)]
pub struct StorageManager {
    pub hosts: Vec<Host>,
    pub groups: Vec<HostGroup>,
    pub tunnel_rules: Vec<TunnelRule>,
}

impl StorageManager {
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn tunnel_rule_count(&self) -> usize {
        self.tunnel_rules.len()
    }
}
