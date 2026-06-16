use crate::domain::host::HostRecord;

/// 主机领域服务。
#[derive(Debug, Default)]
pub struct HostService;

impl HostService {
    pub fn create_localhost_example(&self) -> HostRecord {
        HostRecord::new("localhost", "127.0.0.1", 22)
    }
}
