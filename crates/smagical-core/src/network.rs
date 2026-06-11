//! 网络路由相关纯领域模型。
//!
//! 这里集中放代理、跳板链和相关可复用资产，避免继续混在 `Host` 本体里膨胀。

use serde::{Deserialize, Deserializer, Serialize};

use crate::{ForwardId, HostId, JumpChainId, ProxyId, TunnelRule};

/// 连接代理配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyProfile {
    /// SOCKS5 代理。
    Socks5 { host: String, port: u16 },
    /// HTTP CONNECT 代理。
    Http { host: String, port: u16 },
}

/// 跳板机链路中的一个节点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpProfile {
    /// 跳板机引用已保存主机，避免在链路里复制整份主机配置。
    pub host_id: HostId,
}

/// 可复用代理资产。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyAsset {
    /// 代理稳定 ID。
    pub id: ProxyId,
    /// 显示名称。
    pub name: String,
    /// 用户标签，方便筛选和分类。
    #[serde(default)]
    pub tags: Vec<String>,
    /// 实际代理协议和地址。
    pub profile: ProxyProfile,
}

/// 可复用跳板链资产。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpChainAsset {
    /// 跳板链稳定 ID。
    pub id: JumpChainId,
    /// 显示名称。
    pub name: String,
    /// 跳板步骤，按顺序进入。
    #[serde(default)]
    pub steps: Vec<JumpProfile>,
}

/// 可复用端口转发资产。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardAsset {
    /// 端口转发稳定 ID。
    pub id: ForwardId,
    /// 显示名称。
    pub name: String,
    /// 用户标签，方便筛选和分类。
    #[serde(default)]
    pub tags: Vec<String>,
    /// 实际端口转发规则。
    pub rule: TunnelRule,
}

/// 主机引用的网络资源选择。
///
/// 主机只保存资源 ID，不复制代理地址、跳板路径和端口转发细节。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostNetworkSelection {
    /// 主机使用的代理资产，按顺序应用。
    #[serde(default)]
    pub proxy_ids: Vec<ProxyId>,
    /// 主机使用的跳板链资产，按顺序应用。
    #[serde(default)]
    pub jump_chain_ids: Vec<JumpChainId>,
    /// 主机绑定的端口转发资产，允许多选。
    #[serde(default)]
    pub forward_ids: Vec<ForwardId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum HostProxyField {
    One(ProxyProfile),
    Many(Vec<ProxyProfile>),
}

pub(crate) fn deserialize_host_proxies<'de, D>(
    deserializer: D,
) -> Result<Vec<ProxyProfile>, D::Error>
where
    D: Deserializer<'de>,
{
    let field = Option::<HostProxyField>::deserialize(deserializer)?;
    Ok(match field {
        Some(HostProxyField::One(proxy)) => vec![proxy],
        Some(HostProxyField::Many(proxies)) => proxies,
        None => Vec::new(),
    })
}
