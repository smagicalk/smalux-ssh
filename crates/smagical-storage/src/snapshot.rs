//! 持久化快照格式与内存存储转换。
//!
//! 快照是导入导出和旧 redb 兼容使用的业务格式。它不是数据库 schema：SQLite 可以拆成
//! 多张表，但导出时仍可以还原成这个结构，方便备份、迁移和测试。

use serde::{Deserialize, Serialize};

use smagical_config::AppConfig;
use smagical_core::{
    CommandHistoryItem, CredentialMetadata, Host, HostGroup, KnownHostEntry, RecentConnection,
    SftpBookmark, Snippet, TunnelRule, WorkspaceState,
};

use super::{StorageManager, ThemeProfileRecord};

/// 持久化快照格式。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct StorageSnapshot {
    /// 应用配置。
    pub(crate) app_config: AppConfig,
    /// 主机集合。
    pub(crate) hosts: Vec<Host>,
    /// 分组集合。
    pub(crate) groups: Vec<HostGroup>,
    /// 凭据元数据集合。
    pub(crate) credentials: Vec<CredentialMetadata>,
    /// Known Hosts 集合。
    pub(crate) known_hosts: Vec<KnownHostEntry>,
    /// 最近连接集合。
    pub(crate) recent_connections: Vec<RecentConnection>,
    /// 命令历史集合。
    pub(crate) command_history: Vec<CommandHistoryItem>,
    /// 快捷命令集合。
    pub(crate) snippets: Vec<Snippet>,
    /// SFTP 书签集合。
    pub(crate) sftp_bookmarks: Vec<SftpBookmark>,
    /// 隧道规则集合。
    pub(crate) tunnel_rules: Vec<TunnelRule>,
    /// 主题资料集合。
    pub(crate) themes: Vec<ThemeProfileRecord>,
    /// 工作区快照。
    pub(crate) workspace: Option<WorkspaceState>,
}

impl From<&StorageManager> for StorageSnapshot {
    fn from(storage: &StorageManager) -> Self {
        Self {
            app_config: storage.app_config.clone(),
            hosts: storage.hosts.clone(),
            groups: storage.groups.clone(),
            credentials: storage.credentials.clone(),
            known_hosts: storage.known_hosts.clone(),
            recent_connections: storage.recent_connections.clone(),
            command_history: storage.command_history.clone(),
            snippets: storage.snippets.clone(),
            sftp_bookmarks: storage.sftp_bookmarks.clone(),
            tunnel_rules: storage.tunnel_rules.clone(),
            themes: storage.themes.clone(),
            workspace: storage.workspace.clone(),
        }
    }
}

impl StorageSnapshot {
    pub(crate) fn into_storage(self) -> StorageManager {
        // command_history 和 tunnel_rules 通过 StorageManager 方法导入，重新套用数量限制、
        // 规范化和去重规则，避免旧快照绕过内存不变量。
        let mut storage = StorageManager {
            app_config: self.app_config,
            hosts: self.hosts,
            groups: self.groups,
            credentials: self.credentials,
            known_hosts: self.known_hosts,
            recent_connections: self.recent_connections,
            command_history: Vec::new(),
            snippets: self.snippets,
            sftp_bookmarks: self.sftp_bookmarks,
            tunnel_rules: Vec::new(),
            themes: self.themes,
            workspace: self.workspace,
        };

        for item in self.command_history {
            storage.add_command_history(item);
        }
        for rule in self.tunnel_rules {
            storage.upsert_tunnel_rule(rule);
        }

        storage
    }
}
