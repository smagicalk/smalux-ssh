//! 网络资源库管理。
//!
//! 这里维护可复用代理、跳板链和端口转发资产。主机只保存这些资产的 ID 引用，
//! 因此删除时必须先检查引用，避免主机留下悬空网络配置。

use uuid::Uuid;

use crate::model::{
    AppState, AppUpdateOutcome, ForwardAsset, ForwardId, HostId, JumpChainAsset, JumpChainId,
    JumpProfile, ProxyAsset, ProxyId, ProxyProfile, TunnelKind, TunnelRule,
};

const NETWORK_ASSET_NAME_LIMIT: usize = 64;

impl AppState {
    /// 创建或更新代理资产。
    pub(in crate::model::app_state) fn save_proxy_asset(
        &mut self,
        proxy_id: Option<ProxyId>,
        name: String,
        proxy_kind: String,
        host: String,
        port: String,
        tags: String,
    ) -> AppUpdateOutcome {
        let name = normalized_name(&name);
        if name.is_empty() {
            return error("代理名称不能为空");
        }
        if let Some(existing_id) = proxy_id {
            if self.storage.proxy_asset_by_id(existing_id).is_none() {
                return error("代理资源不存在，无法更新");
            }
        }
        if self
            .storage
            .proxy_assets
            .iter()
            .any(|asset| asset.name == name && Some(asset.id) != proxy_id)
        {
            return error("代理名称已存在");
        }

        let host = host.trim().to_owned();
        if host.is_empty() {
            return error("代理地址不能为空");
        }
        let Some(port) = parse_port(&port) else {
            return error("代理端口无效");
        };
        let profile = match proxy_kind.trim() {
            "Http" | "http" | "HTTP" | "HTTP CONNECT" => ProxyProfile::Http { host, port },
            _ => ProxyProfile::Socks5 { host, port },
        };

        self.storage.upsert_proxy_asset(ProxyAsset {
            id: proxy_id.unwrap_or_else(|| ProxyId(Uuid::new_v4())),
            name,
            tags: parse_tags(&tags),
            profile,
        });
        changed()
    }

    /// 创建或更新跳板链资产。
    pub(in crate::model::app_state) fn save_jump_chain_asset(
        &mut self,
        chain_id: Option<JumpChainId>,
        name: String,
        host_ids: Vec<HostId>,
    ) -> AppUpdateOutcome {
        let name = normalized_name(&name);
        if name.is_empty() {
            return error("跳板链名称不能为空");
        }
        if let Some(existing_id) = chain_id {
            if self.storage.jump_chain_asset_by_id(existing_id).is_none() {
                return error("跳板链资源不存在，无法更新");
            }
        }
        if self
            .storage
            .jump_chain_assets
            .iter()
            .any(|asset| asset.name == name && Some(asset.id) != chain_id)
        {
            return error("跳板链名称已存在");
        }

        let mut normalized_host_ids = Vec::new();
        for host_id in host_ids {
            if !normalized_host_ids.contains(&host_id) {
                normalized_host_ids.push(host_id);
            }
        }
        if normalized_host_ids.is_empty() {
            return error("跳板链至少需要一个主机节点");
        }
        if normalized_host_ids
            .iter()
            .any(|host_id| !self.storage.hosts.iter().any(|host| host.id == *host_id))
        {
            return error("跳板链包含不存在的主机");
        }

        self.storage.upsert_jump_chain_asset(JumpChainAsset {
            id: chain_id.unwrap_or_else(|| JumpChainId(Uuid::new_v4())),
            name,
            steps: normalized_host_ids
                .into_iter()
                .map(|host_id| JumpProfile { host_id })
                .collect(),
        });
        changed()
    }

    /// 创建或更新端口转发资产。
    #[allow(clippy::too_many_arguments)]
    pub(in crate::model::app_state) fn save_forward_asset(
        &mut self,
        forward_id: Option<ForwardId>,
        name: String,
        kind: String,
        bind_host: String,
        bind_port: String,
        target_host: String,
        target_port: String,
        tags: String,
        auto_start: bool,
    ) -> AppUpdateOutcome {
        let name = normalized_name(&name);
        if name.is_empty() {
            return error("转发名称不能为空");
        }
        if let Some(existing_id) = forward_id {
            if self.storage.forward_asset_by_id(existing_id).is_none() {
                return error("转发资源不存在，无法更新");
            }
        }
        if self
            .storage
            .forward_assets
            .iter()
            .any(|asset| asset.name == name && Some(asset.id) != forward_id)
        {
            return error("转发名称已存在");
        }

        let Some(bind_port) = parse_port(&bind_port) else {
            return error("绑定端口无效");
        };
        let tunnel_kind = tunnel_kind_from_key(&kind);
        let target_port =
            if matches!(tunnel_kind, TunnelKind::Dynamic) && target_port.trim().is_empty() {
                0
            } else {
                let Some(port) = parse_port(&target_port) else {
                    return error("目标端口无效");
                };
                port
            };
        let rule = TunnelRule {
            name: name.clone(),
            kind: tunnel_kind,
            bind_host,
            bind_port,
            target_host,
            target_port,
            auto_start,
        }
        .normalized();
        if let Err(error) = rule.validate() {
            return AppUpdateOutcome {
                error: Some(format!("转发配置无效：{error:?}")),
                ..AppUpdateOutcome::default()
            };
        }

        self.storage.upsert_forward_asset(ForwardAsset {
            id: forward_id.unwrap_or_else(|| ForwardId(Uuid::new_v4())),
            name,
            tags: parse_tags(&tags),
            rule,
        });
        changed()
    }

    /// 删除代理资产；仍被主机引用时返回使用位置。
    pub(in crate::model::app_state) fn remove_proxy_asset(
        &mut self,
        proxy_id: ProxyId,
    ) -> AppUpdateOutcome {
        if self.storage.proxy_asset_by_id(proxy_id).is_none() {
            return error("代理资源不存在，无法删除");
        }
        let used_by = host_names_for_ids(self, self.storage.proxy_asset_host_ids(proxy_id));
        if !used_by.is_empty() {
            return referenced_error("代理资源", &used_by);
        }
        if self.storage.remove_proxy_asset(proxy_id) {
            changed()
        } else {
            error("代理资源不存在，无法删除")
        }
    }

    /// 删除跳板链资产；仍被主机引用时返回使用位置。
    pub(in crate::model::app_state) fn remove_jump_chain_asset(
        &mut self,
        chain_id: JumpChainId,
    ) -> AppUpdateOutcome {
        if self.storage.jump_chain_asset_by_id(chain_id).is_none() {
            return error("跳板链资源不存在，无法删除");
        }
        let used_by = host_names_for_ids(self, self.storage.jump_chain_asset_host_ids(chain_id));
        if !used_by.is_empty() {
            return referenced_error("跳板链资源", &used_by);
        }
        if self.storage.remove_jump_chain_asset(chain_id) {
            changed()
        } else {
            error("跳板链资源不存在，无法删除")
        }
    }

    /// 删除端口转发资产；仍被主机引用时返回使用位置。
    pub(in crate::model::app_state) fn remove_forward_asset(
        &mut self,
        forward_id: ForwardId,
    ) -> AppUpdateOutcome {
        if self.storage.forward_asset_by_id(forward_id).is_none() {
            return error("转发资源不存在，无法删除");
        }
        let used_by = host_names_for_ids(self, self.storage.forward_asset_host_ids(forward_id));
        if !used_by.is_empty() {
            return referenced_error("转发资源", &used_by);
        }
        if self.storage.remove_forward_asset(forward_id) {
            changed()
        } else {
            error("转发资源不存在，无法删除")
        }
    }
}

fn normalized_name(name: &str) -> String {
    name.trim().chars().take(NETWORK_ASSET_NAME_LIMIT).collect()
}

fn parse_port(port: &str) -> Option<u16> {
    let port = port.trim().parse::<u16>().ok()?;
    (port > 0).then_some(port)
}

fn parse_tags(tags: &str) -> Vec<String> {
    tags.split(|ch: char| ch == ',' || ch == ';' || ch == '，' || ch == '；' || ch.is_whitespace())
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn tunnel_kind_from_key(kind: &str) -> TunnelKind {
    match kind.trim() {
        "Remote" | "remote" => TunnelKind::Remote,
        "Dynamic" | "dynamic" => TunnelKind::Dynamic,
        _ => TunnelKind::Local,
    }
}

fn host_names_for_ids(state: &AppState, host_ids: Vec<HostId>) -> Vec<String> {
    host_ids
        .into_iter()
        .map(|host_id| {
            state
                .storage
                .hosts
                .iter()
                .find(|host| host.id == host_id)
                .map(|host| host.name.clone())
                .unwrap_or_else(|| host_id.0.to_string())
        })
        .collect()
}

fn referenced_error(kind: &str, host_names: &[String]) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!(
            "{kind}正在被主机使用，不能删除。使用位置：{}",
            host_names.join("、")
        )),
        ..AppUpdateOutcome::default()
    }
}

fn changed() -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}

fn error(message: &str) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(message.to_owned()),
        ..AppUpdateOutcome::default()
    }
}
