//! 可复用网络资产的内存索引操作。

use smagical_core::{ForwardAsset, ForwardId, JumpChainAsset, JumpChainId, ProxyAsset, ProxyId};

use super::StorageManager;

impl StorageManager {
    /// 保存或更新可复用代理资产。
    pub fn upsert_proxy_asset(&mut self, asset: ProxyAsset) {
        if let Some(existing) = self
            .proxy_assets
            .iter_mut()
            .find(|existing| existing.id == asset.id)
        {
            *existing = asset;
        } else {
            self.proxy_assets.push(asset);
        }
    }

    /// 按 ID 查找代理资产。
    pub fn proxy_asset_by_id(&self, proxy_id: ProxyId) -> Option<&ProxyAsset> {
        self.proxy_assets.iter().find(|asset| asset.id == proxy_id)
    }

    /// 返回引用指定代理资产的主机 ID。
    pub fn proxy_asset_host_ids(&self, proxy_id: ProxyId) -> Vec<smagical_core::HostId> {
        self.hosts
            .iter()
            .filter(|host| host.network.proxy_ids.contains(&proxy_id))
            .map(|host| host.id)
            .collect()
    }

    /// 判断指定代理资产是否仍被主机引用。
    pub fn proxy_asset_is_referenced(&self, proxy_id: ProxyId) -> bool {
        !self.proxy_asset_host_ids(proxy_id).is_empty()
    }

    /// 删除代理资产。
    pub fn remove_proxy_asset(&mut self, proxy_id: ProxyId) -> bool {
        if self.proxy_asset_is_referenced(proxy_id) {
            return false;
        }
        let before = self.proxy_assets.len();
        self.proxy_assets.retain(|asset| asset.id != proxy_id);
        before != self.proxy_assets.len()
    }

    /// 保存或更新可复用跳板链资产。
    pub fn upsert_jump_chain_asset(&mut self, asset: JumpChainAsset) {
        if let Some(existing) = self
            .jump_chain_assets
            .iter_mut()
            .find(|existing| existing.id == asset.id)
        {
            *existing = asset;
        } else {
            self.jump_chain_assets.push(asset);
        }
    }

    /// 按 ID 查找跳板链资产。
    pub fn jump_chain_asset_by_id(&self, chain_id: JumpChainId) -> Option<&JumpChainAsset> {
        self.jump_chain_assets
            .iter()
            .find(|asset| asset.id == chain_id)
    }

    /// 返回引用指定跳板链资产的主机 ID。
    pub fn jump_chain_asset_host_ids(&self, chain_id: JumpChainId) -> Vec<smagical_core::HostId> {
        self.hosts
            .iter()
            .filter(|host| host.network.jump_chain_ids.contains(&chain_id))
            .map(|host| host.id)
            .collect()
    }

    /// 判断指定跳板链资产是否仍被主机引用。
    pub fn jump_chain_asset_is_referenced(&self, chain_id: JumpChainId) -> bool {
        !self.jump_chain_asset_host_ids(chain_id).is_empty()
    }

    /// 删除跳板链资产。
    pub fn remove_jump_chain_asset(&mut self, chain_id: JumpChainId) -> bool {
        if self.jump_chain_asset_is_referenced(chain_id) {
            return false;
        }
        let before = self.jump_chain_assets.len();
        self.jump_chain_assets.retain(|asset| asset.id != chain_id);
        before != self.jump_chain_assets.len()
    }

    /// 保存或更新可复用端口转发资产。
    pub fn upsert_forward_asset(&mut self, asset: ForwardAsset) {
        if let Some(existing) = self
            .forward_assets
            .iter_mut()
            .find(|existing| existing.id == asset.id)
        {
            *existing = asset;
        } else {
            self.forward_assets.push(asset);
        }
    }

    /// 按 ID 查找端口转发资产。
    pub fn forward_asset_by_id(&self, forward_id: ForwardId) -> Option<&ForwardAsset> {
        self.forward_assets
            .iter()
            .find(|asset| asset.id == forward_id)
    }

    /// 返回引用指定端口转发资产的主机 ID。
    pub fn forward_asset_host_ids(&self, forward_id: ForwardId) -> Vec<smagical_core::HostId> {
        self.hosts
            .iter()
            .filter(|host| host.network.forward_ids.contains(&forward_id))
            .map(|host| host.id)
            .collect()
    }

    /// 判断指定端口转发资产是否仍被主机引用。
    pub fn forward_asset_is_referenced(&self, forward_id: ForwardId) -> bool {
        !self.forward_asset_host_ids(forward_id).is_empty()
    }

    /// 删除端口转发资产。
    pub fn remove_forward_asset(&mut self, forward_id: ForwardId) -> bool {
        if self.forward_asset_is_referenced(forward_id) {
            return false;
        }
        let before = self.forward_assets.len();
        self.forward_assets.retain(|asset| asset.id != forward_id);
        before != self.forward_assets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::{
        AuthProfile, Host, HostId, HostNetworkSelection, JumpProfile, ProxyAuth, ProxyProfile,
        SecretRef, TunnelKind, TunnelRule,
    };
    use uuid::Uuid;

    fn sample_proxy_asset(id: ProxyId, name: &str, host: &str, port: u16) -> ProxyAsset {
        ProxyAsset {
            id,
            name: name.to_owned(),
            tags: vec!["shared".to_owned()],
            profile: ProxyProfile::Socks5 {
                host: host.to_owned(),
                port,
                auth: ProxyAuth::None,
                remote_dns: false,
            },
        }
    }

    fn sample_host_with_network(
        id: HostId,
        proxy_ids: Vec<ProxyId>,
        jump_chain_ids: Vec<JumpChainId>,
        forward_ids: Vec<ForwardId>,
    ) -> Host {
        Host {
            id,
            name: "production".to_owned(),
            group_id: None,
            icon_key: "server".to_owned(),
            tags: vec!["prod".to_owned()],
            address: "prod.example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Password {
                username: "ops".to_owned(),
                secret: SecretRef("password:ops".to_owned()),
            },
            network: HostNetworkSelection {
                proxy_ids,
                jump_chain_ids,
                forward_ids,
            },
            proxies: Vec::new(),
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    fn sample_jump_chain_asset(id: JumpChainId, host_id: HostId) -> JumpChainAsset {
        JumpChainAsset {
            id,
            name: "prod-chain".to_owned(),
            steps: vec![JumpProfile {
                host_id,
                username_override: None,
                port_override: None,
                alias: None,
            }],
            stop_on_failure: true,
        }
    }

    fn sample_forward_asset(id: ForwardId, name: &str) -> ForwardAsset {
        ForwardAsset {
            id,
            name: name.to_owned(),
            tags: vec!["shared".to_owned()],
            rule: TunnelRule {
                name: name.to_owned(),
                kind: TunnelKind::Local,
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 15432,
                target_host: "db.internal".to_owned(),
                target_port: 5432,
                auto_start: false,
                exit_on_failure: false,
            },
            exit_on_failure: false,
        }
    }

    #[test]
    fn proxy_assets_can_be_found_and_removed_by_id() {
        let mut storage = StorageManager::default();
        let proxy_id = ProxyId(Uuid::new_v4());

        storage.upsert_proxy_asset(sample_proxy_asset(proxy_id, "办公出口", "127.0.0.1", 1080));

        assert!(storage.proxy_asset_by_id(proxy_id).is_some());
        assert!(storage.remove_proxy_asset(proxy_id));
        assert!(storage.proxy_asset_by_id(proxy_id).is_none());
        assert!(!storage.remove_proxy_asset(proxy_id));
    }

    #[test]
    fn proxy_asset_removal_is_blocked_when_host_references_it() {
        let mut storage = StorageManager::default();
        let proxy_id = ProxyId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());

        storage.upsert_proxy_asset(sample_proxy_asset(proxy_id, "办公出口", "127.0.0.1", 1080));
        storage.upsert_host(sample_host_with_network(
            host_id,
            vec![proxy_id],
            Vec::new(),
            Vec::new(),
        ));

        assert_eq!(storage.proxy_asset_host_ids(proxy_id), vec![host_id]);
        assert!(storage.proxy_asset_is_referenced(proxy_id));
        assert!(!storage.remove_proxy_asset(proxy_id));
        assert!(storage.proxy_asset_by_id(proxy_id).is_some());
    }

    #[test]
    fn jump_chain_assets_can_be_found_and_removed_by_id() {
        let mut storage = StorageManager::default();
        let chain_id = JumpChainId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());

        storage.upsert_jump_chain_asset(sample_jump_chain_asset(chain_id, host_id));

        assert!(storage.jump_chain_asset_by_id(chain_id).is_some());
        assert!(storage.remove_jump_chain_asset(chain_id));
        assert!(storage.jump_chain_asset_by_id(chain_id).is_none());
        assert!(!storage.remove_jump_chain_asset(chain_id));
    }

    #[test]
    fn jump_chain_asset_removal_is_blocked_when_host_references_it() {
        let mut storage = StorageManager::default();
        let chain_id = JumpChainId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());

        storage.upsert_jump_chain_asset(sample_jump_chain_asset(chain_id, host_id));
        storage.upsert_host(sample_host_with_network(
            HostId(Uuid::new_v4()),
            Vec::new(),
            vec![chain_id],
            Vec::new(),
        ));

        assert_eq!(storage.jump_chain_asset_host_ids(chain_id).len(), 1);
        assert!(storage.jump_chain_asset_is_referenced(chain_id));
        assert!(!storage.remove_jump_chain_asset(chain_id));
        assert!(storage.jump_chain_asset_by_id(chain_id).is_some());
    }

    #[test]
    fn forward_assets_can_be_found_and_removed_by_id() {
        let mut storage = StorageManager::default();
        let forward_id = ForwardId(Uuid::new_v4());

        storage.upsert_forward_asset(sample_forward_asset(forward_id, "local-db"));

        assert!(storage.forward_asset_by_id(forward_id).is_some());
        assert!(storage.remove_forward_asset(forward_id));
        assert!(storage.forward_asset_by_id(forward_id).is_none());
        assert!(!storage.remove_forward_asset(forward_id));
    }

    #[test]
    fn forward_asset_removal_is_blocked_when_host_references_it() {
        let mut storage = StorageManager::default();
        let forward_id = ForwardId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());

        storage.upsert_forward_asset(sample_forward_asset(forward_id, "local-db"));
        storage.upsert_host(sample_host_with_network(
            host_id,
            Vec::new(),
            Vec::new(),
            vec![forward_id],
        ));

        assert_eq!(storage.forward_asset_host_ids(forward_id), vec![host_id]);
        assert!(storage.forward_asset_is_referenced(forward_id));
        assert!(!storage.remove_forward_asset(forward_id));
        assert!(storage.forward_asset_by_id(forward_id).is_some());
    }
}
