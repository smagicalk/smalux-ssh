//! 主机与分组的内存索引操作。

use crate::model::{Host, HostGroup};

use super::StorageManager;

impl StorageManager {
    /// 保存或更新主机配置。
    pub fn upsert_host(&mut self, host: Host) {
        if let Some(existing) = self
            .hosts
            .iter_mut()
            .find(|existing| existing.id == host.id)
        {
            *existing = host;
        } else {
            self.hosts.push(host);
        }
    }

    /// 保存或更新分组配置。
    pub fn upsert_group(&mut self, group: HostGroup) {
        if let Some(existing) = self
            .groups
            .iter_mut()
            .find(|existing| existing.id == group.id)
        {
            *existing = group;
        } else {
            self.groups.push(group);
        }
    }

    /// 按关键字搜索主机名称、地址和标签。
    pub fn search_hosts(&self, query: &str) -> Vec<&Host> {
        let query = query.trim().to_lowercase();

        if query.is_empty() {
            return self.hosts.iter().collect();
        }

        self.hosts
            .iter()
            .filter(|host| {
                host.name.to_lowercase().contains(&query)
                    || host.address.to_lowercase().contains(&query)
                    || host
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthProfile, HostId, SecretRef};
    use uuid::Uuid;

    fn sample_host_with(id: HostId, name: &str, address: &str, tags: &[&str]) -> Host {
        Host {
            id,
            name: name.to_owned(),
            group_id: None,
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            address: address.to_owned(),
            port: 22,
            auth: AuthProfile::Password {
                username: "ops".to_owned(),
                secret: SecretRef("password:ops".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn upsert_host_replaces_existing_host() {
        let mut storage = StorageManager::default();
        let host_id = HostId(Uuid::new_v4());

        storage.upsert_host(sample_host_with(
            host_id,
            "old-name",
            "old.example.com",
            &["legacy"],
        ));
        storage.upsert_host(sample_host_with(
            host_id,
            "new-name",
            "new.example.com",
            &["prod"],
        ));

        assert_eq!(storage.host_count(), 1);
        assert_eq!(storage.hosts[0].name, "new-name");
        assert_eq!(storage.hosts[0].address, "new.example.com");
        assert_eq!(storage.hosts[0].tags, vec!["prod"]);
    }

    #[test]
    fn search_hosts_matches_name_address_and_tags() {
        let mut storage = StorageManager::default();

        storage.upsert_host(sample_host_with(
            HostId(Uuid::new_v4()),
            "Production API",
            "api.example.com",
            &["prod", "linux"],
        ));
        storage.upsert_host(sample_host_with(
            HostId(Uuid::new_v4()),
            "Jump Box",
            "jump.internal",
            &["bastion"],
        ));

        assert_eq!(storage.search_hosts("production").len(), 1);
        assert_eq!(storage.search_hosts("internal").len(), 1);
        assert_eq!(storage.search_hosts("BASTION").len(), 1);
        assert_eq!(storage.search_hosts("").len(), 2);
    }
}
