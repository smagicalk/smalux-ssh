use crate::domain::host::HostRecord;
use crate::services::host_service::HostService;

/// 无界面依赖的核心运行态。
#[derive(Debug, Default)]
pub struct CoreState {
    pub hosts: Vec<HostRecord>,
    pub host_service: HostService,
}

impl CoreState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_example_host(&mut self) {
        let host = self.host_service.create_localhost_example();
        self.hosts.push(host);
    }
}
