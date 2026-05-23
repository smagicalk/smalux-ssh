//! 主机列表展示行类型。

/// 主机列表展示行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct HostViewModel {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub auth: &'static str,
    pub group: String,
    pub tags: String,
    pub status: &'static str,
}
