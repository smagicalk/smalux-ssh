use serde::{Deserialize, Serialize};

/// 主机在线状态枚举。
///
/// 统一管理主机健康状态标识，避免魔法字符串散落代码各处。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HostStatus {
    /// 正常在线
    Online,
    /// 告警 / 高延迟
    Warning,
    /// 错误 / 不可达
    Error,
    /// 离线
    #[default]
    Offline,
}

impl HostStatus {
    /// 返回对应的静态字符串标识（供 Slint 绑定层使用）。
    pub fn as_str(&self) -> &'static str {
        match self {
            HostStatus::Online  => "online",
            HostStatus::Warning => "warning",
            HostStatus::Error   => "error",
            HostStatus::Offline => "offline",
        }
    }
}

impl std::fmt::Display for HostStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for HostStatus {
    fn from(s: &str) -> Self {
        match s {
            "online"  => HostStatus::Online,
            "warning" => HostStatus::Warning,
            "error"   => HostStatus::Error,
            _         => HostStatus::Offline,
        }
    }
}

impl From<String> for HostStatus {
    fn from(s: String) -> Self {
        HostStatus::from(s.as_str())
    }
}

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
    /// 关联的认证凭据唯一标识符 (None 表示使用全局默认或无凭据)。
    pub credential_id: Option<String>,
    /// 主机在线健康状态。
    pub status: HostStatus,
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
            credential_id: None,
            status: HostStatus::Online,
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

    /// 指定关联凭据 ID 创建或配置主机记录。
    pub fn with_credential(mut self, credential_id: impl Into<String>) -> Self {
        self.credential_id = Some(credential_id.into());
        self
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
