use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 主机记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRecord {
    /// 主机记录的稳定唯一标识。
    pub id: Uuid,
    /// 面向用户显示的主机名称。
    pub name: String,
    /// 主机名或 IP 地址。
    pub address: String,
    /// SSH 服务端口。
    pub port: u16,
}

impl HostRecord {
    /// 使用随机 UUID 创建一条主机记录。
    pub fn new(name: impl Into<String>, address: impl Into<String>, port: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            address: address.into(),
            port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_assigns_connection_fields_and_a_non_nil_id() {
        let host = HostRecord::new("Production", "ssh.example.com", 2202);

        assert_ne!(host.id, Uuid::nil());
        assert_eq!(host.name, "Production");
        assert_eq!(host.address, "ssh.example.com");
        assert_eq!(host.port, 2202);
    }
}
