//! 本地持久化状态的内存入口。
//!
//! SQLite 是当前主存储后端，但应用内部仍通过 `StorageManager` 读写内存快照。这样 UI、
//! 状态机和测试可以操作同一套结构，落盘、备份、导入导出由 storage backend 负责。

mod credentials;
mod history;
mod hosts;
mod known_hosts;
mod network_assets;
mod persistence;
mod sftp;
mod snapshot;
mod snippets;
mod sqlite;
mod themes;
mod tunnels;
mod workspace;

use serde::{Deserialize, Serialize};
use smagical_config::AppConfig;
use smagical_core::{
    CommandHistoryItem, CredentialGroup, CredentialInspection, CredentialMetadata, ForwardAsset,
    Host, HostGroup, JumpChainAsset, KnownHostEntry, ProxyAsset, RecentConnection, SecretRecord,
    SftpBookmark, Snippet, SnippetGroup, TunnelRule, WorkspaceState,
};

pub use persistence::{RedbStorage, StoragePersistenceError};
pub use sqlite::{LegacyImportOutcome, SqliteStorage};

pub(crate) const DEFAULT_RECENT_LIMIT: usize = 20;
pub(crate) const DEFAULT_COMMAND_HISTORY_LIMIT: usize = 500;

/// 主机资产与隧道规则的管理器。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StorageManager {
    /// 应用配置快照，和业务数据一起保存，便于备份恢复。
    pub app_config: AppConfig,
    /// 已保存主机。
    pub hosts: Vec<Host>,
    /// 树形主机分组。
    pub groups: Vec<HostGroup>,
    /// 可复用代理资产。
    pub proxy_assets: Vec<ProxyAsset>,
    /// 可复用跳板链资产。
    pub jump_chain_assets: Vec<JumpChainAsset>,
    /// 可复用端口转发资产。
    pub forward_assets: Vec<ForwardAsset>,
    /// 凭据元数据，不包含明文秘密。
    pub credentials: Vec<CredentialMetadata>,
    /// 凭据内容解析缓存，不包含密码明文。
    pub credential_inspections: Vec<CredentialInspection>,
    /// 密钥页树形分组。
    pub credential_groups: Vec<CredentialGroup>,
    /// 本地安全存储记录，可能是未加密开发期 payload，也可能是外部凭据库引用。
    pub secrets: Vec<SecretRecord>,
    /// Known Hosts 安全记录。
    pub known_hosts: Vec<KnownHostEntry>,
    /// 最近连接列表。
    pub recent_connections: Vec<RecentConnection>,
    /// 命令历史。
    pub command_history: Vec<CommandHistoryItem>,
    /// 快捷命令片段。
    pub snippets: Vec<Snippet>,
    /// 快捷命令片段分组。
    pub snippet_groups: Vec<SnippetGroup>,
    /// SFTP 书签。
    pub sftp_bookmarks: Vec<SftpBookmark>,
    /// SSH 隧道规则。
    pub tunnel_rules: Vec<TunnelRule>,
    /// 可导入/导出的主题资料。
    pub themes: Vec<ThemeProfileRecord>,
    /// 工作区恢复快照。
    pub workspace: Option<WorkspaceState>,
}

/// 可导入、导出和复用的主题资料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeProfileRecord {
    /// 主题 profile 名称。
    pub name: String,
    /// 原始主题配置 TOML，保留导入文件的可导出形态。
    pub profile_toml: String,
    /// 是否为内置主题导出的记录。内置记录通常不允许删除。
    pub builtin: bool,
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

    /// 代理资产数量。
    pub fn proxy_asset_count(&self) -> usize {
        self.proxy_assets.len()
    }

    /// 跳板链资产数量。
    pub fn jump_chain_asset_count(&self) -> usize {
        self.jump_chain_assets.len()
    }

    /// 端口转发资产数量。
    pub fn forward_asset_count(&self) -> usize {
        self.forward_assets.len()
    }

    /// 密钥分组数量。
    pub fn credential_group_count(&self) -> usize {
        self.credential_groups.len()
    }

    /// 安全存储记录数量。
    pub fn secret_count(&self) -> usize {
        self.secrets.len()
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

    /// 快捷命令分组数量。
    pub fn snippet_group_count(&self) -> usize {
        self.snippet_groups.len()
    }

    /// SFTP 书签数量。
    pub fn sftp_bookmark_count(&self) -> usize {
        self.sftp_bookmarks.len()
    }

    /// 已保存隧道规则数量。
    pub fn tunnel_rule_count(&self) -> usize {
        self.tunnel_rules.len()
    }

    /// 已保存主题资料数量。
    pub fn theme_count(&self) -> usize {
        self.themes.len()
    }

    /// 当前保存的工作区标签页数量。
    pub fn workspace_tab_count(&self) -> usize {
        self.workspace
            .as_ref()
            .map(|workspace| workspace.tabs.len())
            .unwrap_or(0)
    }

    /// 是否没有任何用户持久化数据。
    pub fn is_empty(&self) -> bool {
        // app_config 也纳入判断，避免旧数据迁移时误删只改过设置的存储。
        self.hosts.is_empty()
            && self.groups.is_empty()
            && self.proxy_assets.is_empty()
            && self.jump_chain_assets.is_empty()
            && self.forward_assets.is_empty()
            && self.credentials.is_empty()
            && self.credential_groups.is_empty()
            && self.secrets.is_empty()
            && self.known_hosts.is_empty()
            && self.recent_connections.is_empty()
            && self.command_history.is_empty()
            && self.snippets.is_empty()
            && self.snippet_groups.is_empty()
            && self.sftp_bookmarks.is_empty()
            && self.tunnel_rules.is_empty()
            && self.themes.is_empty()
            && self.workspace.is_none()
            && self.app_config == AppConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::{
        AuthProfile, CommandHistoryId, ForwardAsset, ForwardId, GroupId, HostId,
        HostNetworkSelection, JumpChainAsset, JumpChainId, ProxyAsset, ProxyAuth, ProxyId,
        RecentConnection, SecretRef, TunnelKind, TunnelRule,
    };
    use uuid::Uuid;

    fn sample_host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "staging".to_owned(),
            group_id: None,
            icon_key: "server".to_owned(),
            tags: vec!["staging".to_owned(), "linux".to_owned()],
            address: "staging.example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Password {
                username: "ops".to_owned(),
                secret: SecretRef("password:ops".to_owned()),
            },
            network: HostNetworkSelection::default(),
            proxies: Vec::new(),
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

    fn sample_proxy_asset() -> ProxyAsset {
        ProxyAsset {
            id: ProxyId(Uuid::new_v4()),
            name: "办公出口".to_owned(),
            tags: vec!["office".to_owned(), "socks".to_owned()],
            profile: smagical_core::ProxyProfile::Socks5 {
                host: "127.0.0.1".to_owned(),
                port: 1080,
                auth: ProxyAuth::None,
                remote_dns: false,
            },
        }
    }

    fn sample_jump_chain_asset() -> JumpChainAsset {
        JumpChainAsset {
            id: JumpChainId(Uuid::new_v4()),
            name: "生产堡垒链".to_owned(),
            steps: vec![smagical_core::JumpProfile {
                host_id: HostId(Uuid::new_v4()),
                username_override: None,
                port_override: None,
                alias: None,
            }],
            stop_on_failure: true,
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
            exit_on_failure: false,
        }
    }

    fn sample_forward_asset() -> ForwardAsset {
        ForwardAsset {
            id: ForwardId(Uuid::new_v4()),
            name: "本地数据库".to_owned(),
            tags: vec!["db".to_owned()],
            rule: sample_tunnel_rule(),
            exit_on_failure: false,
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
        storage.upsert_proxy_asset(sample_proxy_asset());
        storage.upsert_jump_chain_asset(sample_jump_chain_asset());
        storage.upsert_forward_asset(sample_forward_asset());
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
        assert_eq!(storage.proxy_asset_count(), 1);
        assert_eq!(storage.jump_chain_asset_count(), 1);
        assert_eq!(storage.forward_asset_count(), 1);
        assert_eq!(storage.credential_count(), 0);
        assert_eq!(storage.secret_count(), 0);
        assert_eq!(storage.known_host_count(), 0);
        assert_eq!(storage.recent_count(), 1);
        assert_eq!(storage.command_history_count(), 1);
        assert_eq!(storage.snippet_count(), 0);
        assert_eq!(storage.tunnel_rule_count(), 1);
        assert_eq!(storage.workspace_tab_count(), 0);
    }
}
