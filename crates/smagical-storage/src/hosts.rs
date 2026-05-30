//! 主机与分组的内存索引操作。
//!
//! 这里维护内存快照中的主机和分组关系。删除主机/分组时必须同步清理最近连接、历史、
//! 书签和片段等引用，避免 UI 看到悬空 ID。

use smagical_core::{GroupId, Host, HostGroup, HostId, SnippetScope};

use super::StorageManager;

impl StorageManager {
    /// 保存或更新主机配置。
    pub fn upsert_host(&mut self, host: Host) {
        // upsert 以稳定 HostId 为准；编辑主机不会改变关联历史和书签。
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

    /// 删除主机，并清理只依赖该主机的本地列表项。
    pub fn remove_host(&mut self, host_id: HostId) -> bool {
        // 先删除主机本身，再根据 removed 决定是否清理关联集合。
        let before = self.hosts.len();
        self.hosts.retain(|host| host.id != host_id);
        let removed = before != self.hosts.len();

        if removed {
            // 只删除强依赖该主机的本地数据；全局片段和分组片段不受影响。
            self.recent_connections
                .retain(|connection| connection.host_id != host_id);
            self.command_history
                .retain(|item| item.host_id != Some(host_id));
            self.sftp_bookmarks
                .retain(|bookmark| bookmark.host_id != host_id);
            self.snippets.retain(|snippet| {
                !matches!(snippet.scope, SnippetScope::Host(scoped_host_id) if scoped_host_id == host_id)
            });
        }

        removed
    }

    /// 保存或更新分组配置。
    pub fn upsert_group(&mut self, group: HostGroup) {
        // 分组 upsert 不自动检查循环引用；创建/编辑流程应在更高层做校验。
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

    /// 递归删除分组、子分组和分组内主机，并清理相关本地索引。
    pub fn remove_group_recursive(&mut self, group_id: GroupId) -> bool {
        if !self.groups.iter().any(|group| group.id == group_id) {
            return false;
        }

        // 广度优先收集所有子分组，避免递归函数在异常深层树上撑爆栈。
        let mut group_ids = vec![group_id];
        let mut cursor = 0;
        while cursor < group_ids.len() {
            let parent_id = group_ids[cursor];
            for child in self
                .groups
                .iter()
                .filter(|group| group.parent_id == Some(parent_id))
            {
                if !group_ids.contains(&child.id) {
                    group_ids.push(child.id);
                }
            }
            cursor += 1;
        }

        // 删除分组内主机会复用 remove_host 的级联清理逻辑。
        let host_ids: Vec<_> = self
            .hosts
            .iter()
            .filter(|host| host.group_id.is_some_and(|id| group_ids.contains(&id)))
            .map(|host| host.id)
            .collect();
        for host_id in host_ids {
            self.remove_host(host_id);
        }

        // 最后删除分组和分组作用域片段。
        self.groups.retain(|group| !group_ids.contains(&group.id));
        self.snippets.retain(|snippet| {
            !matches!(snippet.scope, SnippetScope::Group(scoped_group_id) if group_ids.contains(&scoped_group_id))
        });

        true
    }

    /// 按关键字搜索主机名称、地址和标签。
    pub fn search_hosts(&self, query: &str) -> Vec<&Host> {
        // 存储层搜索保持轻量，不使用本地化认证/状态文本；UI 展示搜索在 view_model 层做。
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
    use smagical_core::{AuthProfile, HostId, SecretRef};
    use uuid::Uuid;

    fn sample_host_with(id: HostId, name: &str, address: &str, tags: &[&str]) -> Host {
        Host {
            id,
            name: name.to_owned(),
            group_id: None,
            icon_key: "server".to_owned(),
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
    fn remove_host_deletes_host_and_host_scoped_records() {
        use smagical_core::{CommandHistoryId, RecentConnection, SftpBookmark, Snippet, SnippetId};

        let mut storage = StorageManager::default();
        let host_id = HostId(Uuid::new_v4());
        storage.upsert_host(sample_host_with(host_id, "prod", "prod.example.com", &[]));
        storage.record_recent_connection(RecentConnection {
            host_id,
            label: "prod".to_owned(),
            connected_at_unix_secs: 1,
        });
        storage.add_command_history(smagical_core::CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command: "uptime".to_owned(),
            working_directory: None,
            exit_code: Some(0),
            started_at_unix_secs: 1,
            duration_ms: Some(1),
        });
        storage.upsert_sftp_bookmark(SftpBookmark {
            host_id,
            label: "home".to_owned(),
            remote_path: "/home/ops".to_owned(),
        });
        storage.upsert_snippet(Snippet {
            id: SnippetId(Uuid::new_v4()),
            name: "uptime".to_owned(),
            description: None,
            command_template: "uptime".to_owned(),
            scope: SnippetScope::Host(host_id),
            variables: Vec::new(),
            last_arguments: Vec::new(),
        });

        assert!(storage.remove_host(host_id));
        assert!(!storage.remove_host(host_id));
        assert_eq!(storage.host_count(), 0);
        assert_eq!(storage.recent_count(), 0);
        assert_eq!(storage.command_history_count(), 0);
        assert_eq!(storage.sftp_bookmark_count(), 0);
        assert_eq!(storage.snippet_count(), 0);
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
