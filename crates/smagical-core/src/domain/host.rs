use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 主机记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRecord {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub port: u16,
}

impl HostRecord {
    pub fn new(name: impl Into<String>, address: impl Into<String>, port: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            address: address.into(),
            port,
        }
    }
}
