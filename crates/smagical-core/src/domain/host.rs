use serde::{Deserialize, Serialize};

/// 主机资产记录模型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRecord {
    /// 主机记录的稳定唯一标识 (如 "1", "host-k8s-w1")。
    pub id: String,
    /// 面向用户显示的主机名称。
    pub name: String,
    /// 主机名或 IP 地址。
    pub address: String,
    /// SSH 服务端口 (默认为 22)。
    pub port: u16,
    /// 所属分组的唯一标识符 (None 表示未分组)。
    pub parent_group_id: Option<String>,
    /// 在线健康状态 ("online", "warning", "error", "offline")。
    pub status: String,
    /// 网络延迟测速结果 (单位: 毫秒)。
    pub ping_ms: i32,
    /// 列表模式下的显示排序权重。
    pub sort_order: i32,
    /// 主机备注说明信息。
    pub notes: String,
}

impl HostRecord {
    /// 创建一条基础主机记录。
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        address: impl Into<String>,
        port: u16,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            address: address.into(),
            port,
            parent_group_id: None,
            status: "online".to_string(),
            ping_ms: 0,
            sort_order: 0,
            notes: String::new(),
        }
    }

    /// 指定所属分组创建主机记录。
    pub fn with_group(
        id: impl Into<String>,
        name: impl Into<String>,
        address: impl Into<String>,
        port: u16,
        group_id: impl Into<String>,
    ) -> Self {
        let mut host = Self::new(id, name, address, port);
        host.parent_group_id = Some(group_id.into());
        host
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_assigns_connection_fields() {
        let host = HostRecord::new("prod-1", "Production", "ssh.example.com", 2202);

        assert_eq!(host.id, "prod-1");
        assert_eq!(host.name, "Production");
        assert_eq!(host.address, "ssh.example.com");
        assert_eq!(host.port, 2202);
        assert!(host.parent_group_id.is_none());
    }

    #[test]
    fn with_group_constructor_assigns_parent_group() {
        let host = HostRecord::with_group("db-1", "DB Primary", "10.0.1.50", 5432, "grp-db");

        assert_eq!(host.id, "db-1");
        assert_eq!(host.parent_group_id, Some("grp-db".to_string()));
    }
}
