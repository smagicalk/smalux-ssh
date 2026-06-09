//! SeaORM + SQLite 存储后端。
//!
//! SQLite 是当前正式持久化实现。外部仍以同步方法调用，本模块内部用单线程 Tokio runtime
//! 执行 SeaORM 异步操作，避免把 async 传播到 UI 状态层。

use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use sea_orm::{
    ActiveValue::Set, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, EntityTrait,
    PaginatorTrait, Statement,
};
use sea_orm_migration::MigratorTrait;
use tokio::runtime::{Builder, Runtime};

use super::{RedbStorage, StorageManager, StoragePersistenceError};

mod entity;
mod mapper;
mod mapper_common;
mod mapper_credentials;
mod mapper_hosts;
mod migration;
mod migration_common;
mod migration_credentials;
mod migration_extensions;
mod migration_history;
mod migration_hosts;
mod migration_settings;
mod migration_snippets;

const SQLITE_STORAGE_FILE_NAME: &str = "smagicalssh.sqlite";
const APP_CONFIG_SETTING_KEY: &str = "app_config";
const DEFAULT_WORKSPACE_KEY: &str = "default";
const SCHEMA_VERSION_KEY: &str = "schema_version";
const CORE_SCHEMA_VERSION: &str = "1";
const LEGACY_DERIVED_MIGRATION_NAME: &str = "migration";

/// 本地 SQLite 存储入口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteStorage {
    /// 数据库文件路径。
    path: PathBuf,
}

impl SqliteStorage {
    /// 使用指定数据库路径创建存储入口。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 返回系统应用数据目录下的默认 SQLite 数据库路径。
    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "smagical", "smagicalssh")
            .map(|dirs| dirs.data_local_dir().join(SQLITE_STORAGE_FILE_NAME))
    }

    /// 使用默认数据库路径创建存储入口。
    pub fn default_store() -> Option<Self> {
        Self::default_path().map(Self::new)
    }

    /// 数据库路径，便于日志和测试定位。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 从 SQLite 读取存储状态；文件不存在时返回空存储。
    pub fn load(&self) -> Result<StorageManager, StoragePersistenceError> {
        block_on_storage(self.load_async())
    }

    /// 用给定内存存储整体替换持久化状态。
    pub fn save(&self, storage: &StorageManager) -> Result<(), StoragePersistenceError> {
        block_on_storage(self.save_async(storage))
    }

    /// 在 SQLite 为空时导入旧 redb 数据，成功后删除 redb 文件。
    pub fn import_legacy_redb_if_empty(
        &self,
        legacy: &RedbStorage,
    ) -> Result<LegacyImportOutcome, StoragePersistenceError> {
        block_on_storage(self.import_legacy_redb_if_empty_async(legacy))
    }

    /// 在目标路径创建可独立加载的 SQLite 备份。
    pub fn backup_to(&self, target: impl AsRef<Path>) -> Result<(), StoragePersistenceError> {
        block_on_storage(self.backup_to_async(target.as_ref()))
    }

    /// 将当前业务状态导出为 TOML 快照。
    pub fn export_snapshot_to(
        &self,
        target: impl AsRef<Path>,
    ) -> Result<(), StoragePersistenceError> {
        let storage = self.load()?;
        export_storage_snapshot(&storage, target.as_ref())
    }

    /// 导入 TOML 业务快照，并替换当前 SQLite 状态。
    pub fn import_snapshot_from(
        &self,
        source: impl AsRef<Path>,
    ) -> Result<(), StoragePersistenceError> {
        let storage = import_storage_snapshot(source.as_ref())?;
        self.save(&storage)
    }

    /// 导入 SQLite 备份数据库，并替换当前 SQLite 状态。
    pub fn import_sqlite_backup_from(
        &self,
        source: impl AsRef<Path>,
    ) -> Result<(), StoragePersistenceError> {
        let source = source.as_ref();
        if !source.exists() {
            return Err(StoragePersistenceError::InvalidData(format!(
                "导入源不存在：{}",
                source.display()
            )));
        }
        if source == self.path {
            return Err(StoragePersistenceError::InvalidData(
                "导入源不能是当前 SQLite 数据库".to_owned(),
            ));
        }

        let backup = SqliteStorage::new(source);
        let storage = backup.load()?;
        self.save(&storage)
    }

    async fn load_async(&self) -> Result<StorageManager, StoragePersistenceError> {
        if !self.path.exists() {
            return Ok(StorageManager::default());
        }

        // 每次打开都先迁移，保证旧数据库能随应用版本自动升级。
        let db = connect_and_migrate(&self.path).await?;
        mapper::load_storage(&db).await
    }

    async fn save_async(&self, storage: &StorageManager) -> Result<(), StoragePersistenceError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 当前采用整体保存：mapper 负责按表替换业务数据，保证内存快照和数据库一致。
        let db = connect_and_migrate(&self.path).await?;
        mapper::save_storage(&db, storage).await
    }

    async fn import_legacy_redb_if_empty_async(
        &self,
        legacy: &RedbStorage,
    ) -> Result<LegacyImportOutcome, StoragePersistenceError> {
        if !legacy.exists() {
            return Ok(LegacyImportOutcome::NoLegacyFile);
        }

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let db = connect_and_migrate(&self.path).await?;
        if has_business_data(&db).await? {
            // SQLite 已有数据时不能自动合并旧 redb，避免覆盖用户新数据。
            return Ok(LegacyImportOutcome::SkippedSqliteNotEmpty);
        }

        let legacy_storage = legacy.load()?;
        if legacy_storage.is_empty() {
            // 空旧库没有迁移价值，直接删除，避免以后重复提示。
            legacy.delete_file()?;
            return Ok(LegacyImportOutcome::DeletedEmptyLegacy);
        }

        // 导入成功后删除旧 redb，完成一次性迁移。
        mapper::save_storage(&db, &legacy_storage).await?;
        legacy.delete_file()?;
        Ok(LegacyImportOutcome::Imported)
    }

    async fn backup_to_async(&self, target: &Path) -> Result<(), StoragePersistenceError> {
        if !self.path.exists() {
            return Err(StoragePersistenceError::InvalidData(format!(
                "SQLite 数据库不存在：{}",
                self.path.display()
            )));
        }
        if target.exists() {
            return Err(StoragePersistenceError::InvalidData(format!(
                "备份目标已存在：{}",
                target.display()
            )));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        let db = connect_and_migrate(&self.path).await?;
        let target_sql = sqlite_string_literal(&target.to_string_lossy());
        // VACUUM INTO 生成一致的 SQLite 文件副本，比直接复制 WAL 模式文件更可靠。
        db.execute_unprepared(&format!("VACUUM INTO {target_sql}"))
            .await?;
        Ok(())
    }
}

/// 旧 redb 快照导入 SQLite 的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyImportOutcome {
    /// 没有发现旧 redb 文件。
    NoLegacyFile,
    /// SQLite 已有业务数据，跳过导入。
    SkippedSqliteNotEmpty,
    /// 旧 redb 存在但为空，已删除。
    DeletedEmptyLegacy,
    /// 旧 redb 数据已导入 SQLite。
    Imported,
}

async fn connect_and_migrate(path: &Path) -> Result<DatabaseConnection, StoragePersistenceError> {
    // mode=rwc 表示文件不存在则创建，存在则读写打开。
    let url = sqlite_connection_url(path, "rwc")?;
    let db = Database::connect(url).await?;
    configure_sqlite(&db).await?;
    repair_legacy_migration_name(&db).await?;
    migration::Migrator::up(&db, None).await?;
    seed_schema_meta(&db).await?;
    Ok(db)
}

async fn configure_sqlite(db: &DatabaseConnection) -> Result<(), StoragePersistenceError> {
    // 外键约束和 WAL 都是连接级设置，每次连接后都需要显式打开。
    db.execute_unprepared("PRAGMA foreign_keys = ON").await?;
    db.execute_unprepared("PRAGMA journal_mode = WAL").await?;
    Ok(())
}

async fn seed_schema_meta(db: &DatabaseConnection) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    // schema_meta 记录核心 schema 版本，后续加密、导出兼容或迁移检查会用到。
    entity::schema_meta::Entity::insert(entity::schema_meta::ActiveModel {
        key: Set(SCHEMA_VERSION_KEY.to_owned()),
        value: Set(CORE_SCHEMA_VERSION.to_owned()),
        updated_at_unix_secs: Set(now),
    })
    .on_conflict(
        sea_orm::sea_query::OnConflict::column(entity::schema_meta::Column::Key)
            .update_columns([
                entity::schema_meta::Column::Value,
                entity::schema_meta::Column::UpdatedAtUnixSecs,
            ])
            .to_owned(),
    )
    .exec(db)
    .await?;

    Ok(())
}

async fn repair_legacy_migration_name(
    db: &DatabaseConnection,
) -> Result<(), StoragePersistenceError> {
    // 旧版本把多个 migration struct 放在同一个模块并使用 DeriveMigrationName，
    // SeaORM 会记录通用版本名 `migration`。显式 migration 名上线后，旧记录会被
    // SeaORM 判定为“当前 Migrator 缺失的迁移”，因此需要先清理这个兼容性脏记录。
    if !sqlite_table_exists(db, "seaql_migrations").await? {
        return Ok(());
    }

    db.execute_unprepared(&format!(
        "DELETE FROM seaql_migrations WHERE version = {}",
        sqlite_string_literal(LEGACY_DERIVED_MIGRATION_NAME)
    ))
    .await?;
    Ok(())
}

async fn sqlite_table_exists(
    db: &DatabaseConnection,
    table_name: &str,
) -> Result<bool, StoragePersistenceError> {
    let statement = Statement::from_string(
        DatabaseBackend::Sqlite,
        format!(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = {} LIMIT 1",
            sqlite_string_literal(table_name)
        ),
    );
    Ok(db.query_one_raw(statement).await?.is_some())
}

async fn has_business_data(db: &DatabaseConnection) -> Result<bool, StoragePersistenceError> {
    // 只看业务表；schema_meta/app_config 不算用户业务数据，否则空库无法导入旧数据。
    Ok(entity::host::Entity::find().count(db).await? > 0
        || entity::host_group::Entity::find().count(db).await? > 0
        || entity::credential::Entity::find().count(db).await? > 0
        || entity::known_host::Entity::find().count(db).await? > 0
        || entity::recent_connection::Entity::find().count(db).await? > 0
        || entity::command_history::Entity::find().count(db).await? > 0
        || entity::snippet::Entity::find().count(db).await? > 0
        || entity::sftp_bookmark::Entity::find().count(db).await? > 0
        || entity::tunnel_rule::Entity::find().count(db).await? > 0
        || entity::workspace_state::Entity::find().count(db).await? > 0)
}

fn block_on_storage<T>(
    future: impl std::future::Future<Output = Result<T, StoragePersistenceError>>,
) -> Result<T, StoragePersistenceError> {
    // 对上层保持同步 API，避免 UI 状态层为了存储被 async 污染。
    runtime()?.block_on(future)
}

fn runtime() -> Result<Runtime, StoragePersistenceError> {
    // 存储操作串行执行，当前线程 runtime 足够，也避免后台线程生命周期复杂化。
    Ok(Builder::new_current_thread().enable_all().build()?)
}

fn current_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn export_storage_snapshot(
    storage: &StorageManager,
    target: &Path,
) -> Result<(), StoragePersistenceError> {
    if target.exists() {
        return Err(StoragePersistenceError::InvalidData(format!(
            "导出目标已存在：{}",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    // 快照导出业务数据，不导出 SQLite 物理文件结构，适合跨版本迁移。
    let payload = toml::to_string(&super::snapshot::StorageSnapshot::from(storage))?;
    fs::write(target, payload)?;
    Ok(())
}

fn import_storage_snapshot(source: &Path) -> Result<StorageManager, StoragePersistenceError> {
    if !source.exists() {
        return Err(StoragePersistenceError::InvalidData(format!(
            "导入源不存在：{}",
            source.display()
        )));
    }

    // 导入后通过 into_storage 重建 StorageManager，重新应用内存层不变量。
    let payload = fs::read_to_string(source)?;
    let snapshot: super::snapshot::StorageSnapshot = toml::from_str(&payload)?;
    Ok(snapshot.into_storage())
}

fn sqlite_string_literal(value: &str) -> String {
    // VACUUM INTO 需要 SQL 字面量路径，单引号必须转义。
    format!("'{}'", value.replace('\'', "''"))
}

fn sqlite_connection_url(path: &Path, mode: &str) -> Result<String, StoragePersistenceError> {
    // sqlx 的 SQLite URL 使用 `/` 分隔符；Windows `Path::display()` 的反斜杠会被误解析。
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let normalized = absolute.to_string_lossy().replace('\\', "/");
    Ok(format!("sqlite://{normalized}?mode={mode}"))
}

async fn clear_entities<E>(db: &DatabaseConnection) -> Result<(), StoragePersistenceError>
where
    E: EntityTrait,
    <E as EntityTrait>::Model: Send + Sync,
{
    // mapper 保存前按表清空，保证删除的内存记录也会从 SQLite 移除。
    E::delete_many().exec(db).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ThemeProfileRecord;
    use smagical_config::HostListModePreference;
    use smagical_core::{
        AgentSource, AuthProfile, BackgroundProfile, CommandHistoryId, CommandHistoryItem,
        CredentialId, CredentialKind, CredentialMetadata, GroupId, Host, HostGroup, HostId,
        ImageSource, KeyAlgorithm, KnownHostEntry, ProxyProfile, RecentConnection,
        SecretMaterialKind, SecretRecord, SecretRef, SessionId, SessionKind, SftpBookmark, Snippet,
        SnippetArgument, SnippetGroup, SnippetGroupId, SnippetId, SnippetScope,
        SnippetSupportTarget, SnippetSupportTargetId, SplitAxis, TunnelKind, TunnelRule,
        WindowState, WorkspaceState, WorkspaceTabSnapshot,
    };
    use uuid::Uuid;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("smagicalssh-{name}-{}.sqlite", Uuid::new_v4()))
    }

    fn temp_redb_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("smagicalssh-{name}-{}.redb", Uuid::new_v4()))
    }

    fn temp_export_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("smagicalssh-{name}-{}.toml", Uuid::new_v4()))
    }

    fn sample_group(id: GroupId) -> HostGroup {
        HostGroup {
            id,
            name: "Production".to_owned(),
            parent_id: None,
        }
    }

    fn sample_host(id: HostId, group_id: GroupId) -> Host {
        Host {
            id,
            name: "production".to_owned(),
            group_id: Some(group_id),
            icon_key: "cloud".to_owned(),
            tags: vec!["prod".to_owned(), "linux".to_owned()],
            address: "prod.example.com".to_owned(),
            port: 2222,
            auth: AuthProfile::Key {
                username: "deploy".to_owned(),
                key: SecretRef("key:deploy".to_owned()),
                passphrase: Some(SecretRef("passphrase:deploy".to_owned())),
            },
            proxy: Some(ProxyProfile::Socks5 {
                host: "127.0.0.1".to_owned(),
                port: 1080,
            }),
            jumps: vec![smagical_core::JumpProfile { host_id: id }],
            theme_override: Some(smagical_core::ThemeProfile {
                name: "Host Dark".to_owned(),
                font_family: "JetBrains Mono".to_owned(),
                font_size: 15.0,
            }),
            background_override: Some(BackgroundProfile {
                enabled: true,
                sources: vec![ImageSource::LocalPath("wallpapers/a.jpg".to_owned())],
                rotation_interval_secs: 60,
                opacity: 0.25,
                blur: 6.0,
            }),
        }
    }

    fn sample_workspace(host_id: HostId) -> WorkspaceState {
        let session_id = SessionId(Uuid::new_v4());
        let mut workspace = WorkspaceState::empty("restore");
        workspace.window = WindowState {
            width: 1600,
            height: 900,
            maximized: true,
        };
        workspace.upsert_tab(WorkspaceTabSnapshot {
            session_id,
            host_id: Some(host_id),
            kind: SessionKind::RemoteCommand {
                command: "uptime".to_owned(),
                history_id: None,
            },
            title: "uptime".to_owned(),
            working_directory: Some("/home/deploy".to_owned()),
        });
        workspace.rebuild_linear_layout(SplitAxis::Horizontal);
        workspace
    }

    fn sample_storage() -> StorageManager {
        let group_id = GroupId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());
        let snippet_id = SnippetId(Uuid::new_v4());
        let snippet_group_id = SnippetGroupId(Uuid::new_v4());
        let credential_id = CredentialId(Uuid::new_v4());
        let mut storage = StorageManager::default();

        storage.app_config.theme.name = "Solarized Dark".to_owned();
        storage.app_config.workspace.host_list_mode = HostListModePreference::Card;
        storage.app_config.background.enabled = true;
        storage.app_config.background.sources =
            vec![ImageSource::LocalPath("wallpapers/a.jpg".to_owned())];

        storage.upsert_group(sample_group(group_id));
        storage.upsert_host(sample_host(host_id, group_id));
        storage.upsert_credential(CredentialMetadata {
            id: credential_id,
            name: "deploy-key".to_owned(),
            kind: CredentialKind::PrivateKey,
            group_id: None,
            username: Some("deploy".to_owned()),
            secret: Some(SecretRef("key:deploy".to_owned())),
            key_algorithm: Some(KeyAlgorithm::Ed25519),
            fingerprint: Some("SHA256:key".to_owned()),
        });
        storage.upsert_secret(SecretRecord::local_plaintext(
            SecretRef("key:deploy".to_owned()),
            SecretMaterialKind::PrivateKey,
            b"-----BEGIN OPENSSH PRIVATE KEY-----\nmock\n-----END OPENSSH PRIVATE KEY-----\n"
                .to_vec(),
        ));
        storage.upsert_known_host(KnownHostEntry {
            host: "prod.example.com".to_owned(),
            port: 2222,
            key_algorithm: KeyAlgorithm::Unknown("ssh-rsa-cert-v01@openssh.com".to_owned()),
            fingerprint: "SHA256:host".to_owned(),
            trusted: true,
        });
        storage.record_recent_connection(RecentConnection {
            host_id,
            label: "production".to_owned(),
            connected_at_unix_secs: 1_700_000_000,
        });
        storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command: "uptime".to_owned(),
            working_directory: Some("/home/deploy".to_owned()),
            exit_code: Some(0),
            started_at_unix_secs: 1_700_000_001,
            duration_ms: Some(30),
        });
        storage.upsert_snippet_group(SnippetGroup {
            id: snippet_group_id,
            name: "服务".to_owned(),
            parent_id: None,
            sort_order: 0,
        });
        let mut snippet = Snippet::with_default_implementation(
            snippet_id,
            "restart service".to_owned(),
            Some("Restart a systemd service".to_owned()),
            SnippetScope::Host(host_id),
            Some(snippet_group_id),
            "systemctl restart {{service}}".to_owned(),
        );
        snippet.variables[0].default_value = Some("sshd".to_owned());
        snippet
            .default_implementation_mut()
            .expect("默认实现应存在")
            .last_arguments = vec![SnippetArgument {
            name: "service".to_owned(),
            value: "nginx".to_owned(),
        }];
        storage.upsert_snippet(snippet);
        storage.upsert_sftp_bookmark(SftpBookmark {
            host_id,
            label: "home".to_owned(),
            remote_path: "/home/deploy".to_owned(),
        });
        storage.upsert_tunnel_rule(TunnelRule {
            name: "dynamic-proxy".to_owned(),
            kind: TunnelKind::Dynamic,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 1080,
            target_host: String::new(),
            target_port: 0,
            auto_start: false,
        });
        storage.upsert_theme(ThemeProfileRecord {
            name: "Imported Theme".to_owned(),
            profile_toml: r#"name = "Imported Theme""#.to_owned(),
            builtin: false,
        });
        storage.save_workspace(sample_workspace(host_id));
        storage
    }

    fn changed_storage() -> StorageManager {
        let mut storage = sample_storage();
        storage.app_config.app_name = "imported".to_owned();
        storage.hosts[0].name = "imported-production".to_owned();
        storage.snippets.clear();
        storage
    }

    #[test]
    fn missing_sqlite_database_loads_empty_storage() {
        let path = temp_db_path("missing");
        let store = SqliteStorage::new(path);

        let storage = store
            .load()
            .expect("missing SQLite database should load empty storage");

        assert_eq!(storage, StorageManager::default());
    }

    #[test]
    fn sqlite_migrations_create_split_domain_tables() {
        let path = temp_db_path("migration-domain-tables");
        let store = SqliteStorage::new(&path);

        store
            .save(&StorageManager::default())
            .expect("empty storage should migrate schema");

        let url = sqlite_connection_url(&path, "rw").expect("SQLite URL should be valid");
        block_on_storage(async {
            let db = Database::connect(url).await?;
            for table in [
                "schema_meta",
                "host_groups",
                "hosts",
                "credentials",
                "credential_groups",
                "credential_inspections",
                "secrets",
                "snippets",
                "snippet_groups",
                "snippet_implementations",
                "snippet_support_targets",
                "command_history",
                "recent_connections",
                "settings",
                "theme_profiles",
                "workspace_state",
                "sftp_bookmarks",
                "tunnel_rules",
            ] {
                assert!(
                    sqlite_table_exists(&db, table).await?,
                    "{table} should exist"
                );
            }
            Ok::<(), StoragePersistenceError>(())
        })
        .expect("migrated domain tables should be queryable");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn core_storage_round_trips_through_business_tables() {
        let path = temp_db_path("roundtrip");
        let store = SqliteStorage::new(&path);
        let storage = sample_storage();

        store.save(&storage).expect("storage should save to SQLite");
        let loaded = store.load().expect("storage should load from SQLite");

        assert_eq!(loaded, storage);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn snippet_groups_and_arguments_round_trip_through_sqlite() {
        let path = temp_db_path("snippet-roundtrip");
        let store = SqliteStorage::new(&path);
        let parent_id = SnippetGroupId(Uuid::new_v4());
        let child_id = SnippetGroupId(Uuid::new_v4());
        let snippet_id = SnippetId(Uuid::new_v4());
        let host_id = HostId(Uuid::new_v4());
        let mut storage = StorageManager::default();

        storage.upsert_snippet_group(SnippetGroup {
            id: parent_id,
            name: "运维".to_owned(),
            parent_id: None,
            sort_order: 0,
        });
        storage.upsert_snippet_group(SnippetGroup {
            id: child_id,
            name: "服务".to_owned(),
            parent_id: Some(parent_id),
            sort_order: 1,
        });
        let mut snippet = Snippet::with_default_implementation(
            snippet_id,
            "重启服务".to_owned(),
            Some("通过 systemd 重启服务".to_owned()),
            SnippetScope::Host(host_id),
            Some(child_id),
            "systemctl restart {{service}} --env {{env}}".to_owned(),
        );
        snippet.variables[0].default_value = Some("sshd".to_owned());
        snippet.variables[1].required = false;
        snippet
            .default_implementation_mut()
            .expect("默认实现应存在")
            .last_arguments = vec![
            SnippetArgument {
                name: "service".to_owned(),
                value: "nginx".to_owned(),
            },
            SnippetArgument {
                name: "env".to_owned(),
                value: "prod".to_owned(),
            },
        ];
        let shared_implementation_id = snippet.implementations[0].id;
        snippet.support_targets.push(SnippetSupportTarget {
            id: SnippetSupportTargetId(Uuid::new_v4()),
            snippet_id,
            target_key: "debian".to_owned(),
            display_name: "Debian".to_owned(),
            implementation_id: shared_implementation_id,
            sort_order: 1,
        });
        storage.upsert_snippet(snippet);

        store.save(&storage).expect("snippets should save");
        let loaded = store.load().expect("snippets should load");

        assert_eq!(loaded.snippet_groups, storage.snippet_groups);
        assert_eq!(loaded.snippets, storage.snippets);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_replaces_removed_rows() {
        let path = temp_db_path("replace");
        let store = SqliteStorage::new(&path);
        let storage = sample_storage();
        store.save(&storage).expect("first save should work");

        let mut smaller = StorageManager::default();
        smaller.app_config = storage.app_config.clone();
        store.save(&smaller).expect("second save should work");
        let loaded = store.load().expect("storage should load");

        assert_eq!(loaded.host_count(), 0);
        assert_eq!(loaded.credential_count(), 0);
        assert_eq!(loaded.secret_count(), 0);
        assert_eq!(loaded.known_host_count(), 0);
        assert_eq!(loaded.snippet_count(), 0);
        assert_eq!(loaded.tunnel_rule_count(), 0);
        assert_eq!(loaded.app_config, smaller.app_config);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sqlite_save_creates_parent_directory() {
        let path = std::env::temp_dir()
            .join(format!("smagicalssh-nested-{}", Uuid::new_v4()))
            .join("state.sqlite");
        let mut storage = StorageManager::default();
        storage.upsert_host(Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            icon_key: "cloud".to_owned(),
            tags: Vec::new(),
            address: "prod.example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                source: AgentSource::Auto,
                key_hint: Some("id_ed25519".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        });
        let store = SqliteStorage::new(&path);

        store
            .save(&storage)
            .expect("save should create parent directories");

        assert!(path.exists());
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn sqlite_save_repairs_legacy_derived_migration_name() {
        let path = temp_db_path("legacy-migration-name");
        let url = sqlite_connection_url(&path, "rwc").expect("SQLite URL should be valid");
        block_on_storage(async {
            let db = Database::connect(url).await?;
            db.execute_unprepared(
                "CREATE TABLE seaql_migrations (version varchar NOT NULL PRIMARY KEY, applied_at integer NOT NULL)",
            )
            .await?;
            db.execute_unprepared(
                "INSERT INTO seaql_migrations (version, applied_at) VALUES ('migration', 0)",
            )
            .await?;
            Ok::<(), StoragePersistenceError>(())
        })
        .expect("legacy migration table should be created");

        let store = SqliteStorage::new(&path);
        store
            .save(&StorageManager::default())
            .expect("save should repair legacy migration name");

        let loaded = store.load().expect("repaired database should load");
        assert!(loaded.is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn default_path_uses_sqlite_file() {
        let path = SqliteStorage::default_path().expect("platform should provide app data path");

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("smagicalssh.sqlite")
        );
    }

    #[test]
    fn imports_legacy_redb_into_empty_sqlite_and_deletes_legacy_file() {
        let sqlite_path = temp_db_path("legacy-import");
        let redb_path = temp_redb_path("legacy-import");
        let sqlite = SqliteStorage::new(&sqlite_path);
        let legacy = RedbStorage::new(&redb_path);
        let storage = sample_storage();

        legacy
            .save(&storage)
            .expect("legacy redb storage should save");

        let outcome = sqlite
            .import_legacy_redb_if_empty(&legacy)
            .expect("legacy storage should import");
        let loaded = sqlite.load().expect("SQLite storage should load");

        assert_eq!(outcome, LegacyImportOutcome::Imported);
        assert_eq!(loaded, storage);
        assert_eq!(loaded.host_count(), 1);
        assert_eq!(loaded.group_count(), 1);
        assert_eq!(loaded.credential_count(), 1);
        assert_eq!(loaded.secret_count(), 1);
        assert_eq!(loaded.command_history_count(), 1);
        assert_eq!(loaded.snippet_count(), 1);
        assert_eq!(loaded.theme_count(), 1);
        assert_eq!(loaded.hosts[0].group_id, Some(loaded.groups[0].id));
        assert_eq!(
            loaded.app_config.workspace.host_list_mode,
            HostListModePreference::Card
        );
        assert!(!redb_path.exists());
        let _ = fs::remove_file(sqlite_path);
    }

    #[test]
    fn legacy_import_skips_non_empty_sqlite_without_deleting_legacy_file() {
        let sqlite_path = temp_db_path("legacy-skip");
        let redb_path = temp_redb_path("legacy-skip");
        let sqlite = SqliteStorage::new(&sqlite_path);
        let legacy = RedbStorage::new(&redb_path);
        let existing = sample_storage();
        let mut legacy_storage = sample_storage();
        legacy_storage.app_config.app_name = "legacy".to_owned();

        sqlite.save(&existing).expect("SQLite storage should save");
        legacy
            .save(&legacy_storage)
            .expect("legacy redb storage should save");

        let outcome = sqlite
            .import_legacy_redb_if_empty(&legacy)
            .expect("legacy import check should succeed");
        let loaded = sqlite.load().expect("SQLite storage should load");

        assert_eq!(outcome, LegacyImportOutcome::SkippedSqliteNotEmpty);
        assert_eq!(loaded, existing);
        assert!(redb_path.exists());
        let _ = fs::remove_file(sqlite_path);
        let _ = fs::remove_file(redb_path);
    }

    #[test]
    fn legacy_import_deletes_empty_redb_without_writing_user_data() {
        let sqlite_path = temp_db_path("legacy-empty");
        let redb_path = temp_redb_path("legacy-empty");
        let sqlite = SqliteStorage::new(&sqlite_path);
        let legacy = RedbStorage::new(&redb_path);

        legacy
            .save(&StorageManager::default())
            .expect("empty legacy redb storage should save");

        let outcome = sqlite
            .import_legacy_redb_if_empty(&legacy)
            .expect("empty legacy storage should be handled");
        let loaded = sqlite.load().expect("SQLite storage should load");

        assert_eq!(outcome, LegacyImportOutcome::DeletedEmptyLegacy);
        assert_eq!(loaded, StorageManager::default());
        assert!(!redb_path.exists());
        let _ = fs::remove_file(sqlite_path);
    }

    #[test]
    fn sqlite_backup_creates_loadable_database_copy() {
        let sqlite_path = temp_db_path("backup-source");
        let backup_path = temp_db_path("backup-target");
        let store = SqliteStorage::new(&sqlite_path);
        let backup = SqliteStorage::new(&backup_path);
        let storage = sample_storage();

        store.save(&storage).expect("source storage should save");
        store
            .backup_to(&backup_path)
            .expect("backup should succeed");
        let loaded = backup.load().expect("backup database should load");

        assert_eq!(loaded, storage);
        let _ = fs::remove_file(sqlite_path);
        let _ = fs::remove_file(backup_path);
    }

    #[test]
    fn sqlite_backup_refuses_to_overwrite_existing_target() {
        let sqlite_path = temp_db_path("backup-existing-source");
        let backup_path = temp_db_path("backup-existing-target");
        let store = SqliteStorage::new(&sqlite_path);
        store
            .save(&sample_storage())
            .expect("source storage should save");
        fs::write(&backup_path, "existing").expect("existing backup marker should write");

        let error = store
            .backup_to(&backup_path)
            .expect_err("backup should not overwrite existing file");

        assert!(error.to_string().contains("备份目标已存在"));
        let _ = fs::remove_file(sqlite_path);
        let _ = fs::remove_file(backup_path);
    }

    #[test]
    fn exports_business_snapshot_as_toml() {
        let sqlite_path = temp_db_path("export-source");
        let export_path = temp_export_path("export-target");
        let store = SqliteStorage::new(&sqlite_path);
        let storage = sample_storage();
        store.save(&storage).expect("source storage should save");

        store
            .export_snapshot_to(&export_path)
            .expect("snapshot export should succeed");
        let payload = fs::read_to_string(&export_path).expect("exported TOML should read");
        let snapshot: super::super::snapshot::StorageSnapshot =
            toml::from_str(&payload).expect("exported TOML should decode");

        assert_eq!(snapshot.into_storage(), storage);
        let _ = fs::remove_file(sqlite_path);
        let _ = fs::remove_file(export_path);
    }

    #[test]
    fn imports_business_snapshot_and_replaces_existing_sqlite_state() {
        let sqlite_path = temp_db_path("import-snapshot-target");
        let export_path = temp_export_path("import-snapshot-source");
        let source_store = SqliteStorage::new(temp_db_path("import-snapshot-source-db"));
        let target_store = SqliteStorage::new(&sqlite_path);
        let source = changed_storage();
        let existing = sample_storage();

        source_store
            .save(&source)
            .expect("source storage should save");
        source_store
            .export_snapshot_to(&export_path)
            .expect("source snapshot should export");
        target_store
            .save(&existing)
            .expect("target storage should save");

        target_store
            .import_snapshot_from(&export_path)
            .expect("snapshot import should succeed");
        let loaded = target_store.load().expect("target storage should load");

        assert_eq!(loaded, source);
        let _ = fs::remove_file(source_store.path());
        let _ = fs::remove_file(sqlite_path);
        let _ = fs::remove_file(export_path);
    }

    #[test]
    fn imports_sqlite_backup_and_replaces_existing_sqlite_state() {
        let source_path = temp_db_path("import-sqlite-source");
        let target_path = temp_db_path("import-sqlite-target");
        let source_store = SqliteStorage::new(&source_path);
        let target_store = SqliteStorage::new(&target_path);
        let source = changed_storage();
        let existing = sample_storage();

        source_store
            .save(&source)
            .expect("source storage should save");
        target_store
            .save(&existing)
            .expect("target storage should save");

        target_store
            .import_sqlite_backup_from(&source_path)
            .expect("SQLite backup import should succeed");
        let loaded = target_store.load().expect("target storage should load");

        assert_eq!(loaded, source);
        let _ = fs::remove_file(source_path);
        let _ = fs::remove_file(target_path);
    }

    #[test]
    fn sqlite_backup_import_rejects_current_database_as_source() {
        let sqlite_path = temp_db_path("import-self");
        let store = SqliteStorage::new(&sqlite_path);
        store
            .save(&sample_storage())
            .expect("storage should save before import");

        let error = store
            .import_sqlite_backup_from(&sqlite_path)
            .expect_err("self import should be rejected");

        assert!(error.to_string().contains("导入源不能是当前 SQLite 数据库"));
        let _ = fs::remove_file(sqlite_path);
    }
}
