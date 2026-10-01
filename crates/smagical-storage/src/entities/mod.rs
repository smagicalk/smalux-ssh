//! SeaORM 统一通用实体定义模块 (Universal Database Entities)。
//!
//! 严格采用跨库通用数据类型 (String / i32 / i64 / bool)，
//! 100% 兼容 SQLite、PostgreSQL、MySQL 等异构存储后端。

pub mod vault_security;
pub mod system_meta;
pub mod host;
pub mod group;
pub mod credential;
pub mod history;
pub mod snapshot;
pub mod snippet;
pub mod snippet_group;
pub mod tunnel;
pub mod config;
pub mod backup_task;
pub mod backup_snapshot;

pub use vault_security::Entity as VaultSecurity;
pub use system_meta::Entity as SystemMeta;
pub use host::Entity as Host;
pub use group::Entity as Group;
pub use credential::Entity as Credential;
pub use history::Entity as History;
pub use snapshot::Entity as HistorySnapshot;
pub use snippet::Entity as Snippet;
pub use snippet_group::Entity as SnippetGroup;
pub use tunnel::Entity as Tunnel;
pub use config::Entity as AppConfig;
pub use backup_task::Entity as BackupTask;
pub use backup_snapshot::Entity as BackupSnapshot;
