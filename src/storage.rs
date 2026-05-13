//! 本地持久化状态的内存入口。
//!
//! 当前先用轻量内存集合承接 UI 和测试；后续接入 redb 后，外部仍通过
//! StorageManager 访问主机、分组、凭据元数据、Known Hosts、最近连接、命令历史、快捷命令、SFTP 书签、隧道规则和工作区，减少存储实现变化的影响。

mod credentials;
mod history;
mod hosts;
mod known_hosts;
mod persistence;
mod sftp;
mod snippets;
mod tunnels;
mod workspace;

use crate::model::{
    CommandHistoryItem, CredentialMetadata, Host, HostGroup, KnownHostEntry, RecentConnection,
    SftpBookmark, Snippet, TunnelRule, WorkspaceState,
};

pub use persistence::{RedbStorage, StoragePersistenceError};

pub(super) const DEFAULT_RECENT_LIMIT: usize = 20;

/// 主机资产与隧道规则的管理器。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StorageManager {
    pub hosts: Vec<Host>,
    pub groups: Vec<HostGroup>,
    pub credentials: Vec<CredentialMetadata>,
    pub known_hosts: Vec<KnownHostEntry>,
    pub recent_connections: Vec<RecentConnection>,
    pub command_history: Vec<CommandHistoryItem>,
    pub snippets: Vec<Snippet>,
    pub sftp_bookmarks: Vec<SftpBookmark>,
    pub tunnel_rules: Vec<TunnelRule>,
    pub workspace: Option<WorkspaceState>,
}

impl StorageManager {
    /// 已保存主机数量。
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    /// 已保存分组数量。
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// 凭据元数据数量。
    pub fn credential_count(&self) -> usize {
        self.credentials.len()
    }

    /// Known Hosts 记录数量。
    pub fn known_host_count(&self) -> usize {
        self.known_hosts.len()
    }

    /// 最近连接数量。
    pub fn recent_count(&self) -> usize {
        self.recent_connections.len()
    }

    /// 命令历史数量。
    pub fn command_history_count(&self) -> usize {
        self.command_history.len()
    }

    /// 快捷命令数量。
    pub fn snippet_count(&self) -> usize {
        self.snippets.len()
    }

    /// SFTP 书签数量。
    pub fn sftp_bookmark_count(&self) -> usize {
        self.sftp_bookmarks.len()
    }

    /// 已保存隧道规则数量。
    pub fn tunnel_rule_count(&self) -> usize {
        self.tunnel_rules.len()
    }

    /// 当前保存的工作区标签页数量。
    pub fn workspace_tab_count(&self) -> usize {
        self.workspace
            .as_ref()
            .map(|workspace| workspace.tabs.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AuthProfile, CommandHistoryId, GroupId, HostId, RecentConnection, SecretRef, TunnelKind,
        TunnelRule,
    };
    use uuid::Uuid;

    fn sample_host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "staging".to_owned(),
            group_id: None,
            tags: vec!["staging".to_owned(), "linux".to_owned()],
            address: "staging.example.com".to_owned(),
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

    fn sample_group() -> HostGroup {
        HostGroup {
            id: GroupId(Uuid::new_v4()),
            name: "servers".to_owned(),
            parent_id: None,
        }
    }

    fn sample_tunnel_rule() -> TunnelRule {
        TunnelRule {
            name: "dynamic-proxy".to_owned(),
            kind: TunnelKind::Dynamic,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 1080,
            target_host: "ignored-for-dynamic".to_owned(),
            target_port: 0,
            auto_start: false,
        }
    }

    #[test]
    fn default_storage_is_empty() {
        let storage = StorageManager::default();

        assert_eq!(storage.host_count(), 0);
        assert_eq!(storage.group_count(), 0);
        assert_eq!(storage.tunnel_rule_count(), 0);
    }

    #[test]
    fn counters_track_inserted_records() {
        let mut storage = StorageManager::default();

        storage.upsert_host(sample_host());
        storage.upsert_group(sample_group());
        storage.upsert_tunnel_rule(sample_tunnel_rule());
        storage.record_recent_connection(RecentConnection {
            host_id: HostId(Uuid::new_v4()),
            label: "staging".to_owned(),
            connected_at_unix_secs: 1_700_000_000,
        });
        storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: None,
            command: "ls".to_owned(),
            working_directory: None,
            exit_code: Some(0),
            started_at_unix_secs: 1_700_000_000,
            duration_ms: Some(5),
        });

        assert_eq!(storage.host_count(), 1);
        assert_eq!(storage.group_count(), 1);
        assert_eq!(storage.credential_count(), 0);
        assert_eq!(storage.known_host_count(), 0);
        assert_eq!(storage.recent_count(), 1);
        assert_eq!(storage.command_history_count(), 1);
        assert_eq!(storage.snippet_count(), 0);
        assert_eq!(storage.tunnel_rule_count(), 1);
        assert_eq!(storage.workspace_tab_count(), 0);
    }
}
