//! 旧 redb 快照存储。
//!
//! SQLite 是当前主存储后端。本模块保留为兼容读取器：启动时可以把旧 redb 数据导入
//! SQLite，然后安全删除旧文件。

use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use redb::{Database, ReadableDatabase, TableDefinition, TableError};
use thiserror::Error;

#[cfg(test)]
use super::DEFAULT_COMMAND_HISTORY_LIMIT;
use super::StorageManager;
use super::snapshot::StorageSnapshot;

const STORAGE_FILE_NAME: &str = "smagicalssh.redb";
const SNAPSHOT_KEY: &str = "current";
const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("storage_snapshots");

/// 本地 redb 存储入口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedbStorage {
    /// 旧 redb 文件路径。
    path: PathBuf,
}

impl RedbStorage {
    /// 使用指定数据库路径创建存储入口。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 返回系统应用数据目录下的默认数据库路径。
    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "smagical", "smagicalssh")
            .map(|dirs| dirs.data_local_dir().join(STORAGE_FILE_NAME))
    }

    /// 使用默认数据库路径创建存储入口。
    pub fn default_store() -> Option<Self> {
        Self::default_path().map(Self::new)
    }

    /// 数据库路径，便于日志和测试定位。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 判断旧 redb 文件是否存在。
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// 从 redb 读取持久化快照；文件或表不存在时返回空存储。
    pub fn load(&self) -> Result<StorageManager, StoragePersistenceError> {
        if !self.path.exists() {
            return Ok(StorageManager::default());
        }

        let db = Database::open(&self.path)?;
        let read_txn = db.begin_read()?;
        let table = match read_txn.open_table(SNAPSHOT_TABLE) {
            Ok(table) => table,
            Err(TableError::TableDoesNotExist(_)) => return Ok(StorageManager::default()),
            Err(error) => return Err(error.into()),
        };

        let Some(bytes) = table.get(SNAPSHOT_KEY)? else {
            return Ok(StorageManager::default());
        };
        // redb 里保存的是 TOML 快照，读取后再通过 into_storage 应用内存不变量。
        let snapshot: StorageSnapshot = toml::from_slice(bytes.value())?;

        Ok(snapshot.into_storage())
    }

    /// 将当前内存存储保存为 redb 快照。
    pub fn save(&self, storage: &StorageManager) -> Result<(), StoragePersistenceError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 该方法主要供旧数据测试和兼容使用；新数据写入应走 SqliteStorage。
        let encoded = toml::to_string(&StorageSnapshot::from(storage))?;
        let db = open_or_create_database(&self.path)?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(SNAPSHOT_TABLE)?;
            table.insert(SNAPSHOT_KEY, encoded.as_bytes())?;
        }
        write_txn.commit()?;

        Ok(())
    }

    /// 删除旧 redb 文件；不存在时返回 false。
    pub fn delete_file(&self) -> Result<bool, StoragePersistenceError> {
        if !self.path.exists() {
            return Ok(false);
        }

        fs::remove_file(&self.path)?;
        Ok(true)
    }
}

fn open_or_create_database(path: &Path) -> Result<Database, StoragePersistenceError> {
    // redb 打开和创建是两个 API，按文件是否存在分支。
    if path.exists() {
        Ok(Database::open(path)?)
    } else {
        Ok(Database::create(path)?)
    }
}

/// 存储快照读写错误。
#[derive(Debug, Error)]
pub enum StoragePersistenceError {
    #[error("数据库错误：{0}")]
    Database(#[from] redb::DatabaseError),
    #[error("事务错误：{0}")]
    Transaction(#[from] redb::TransactionError),
    #[error("表错误：{0}")]
    Table(#[from] redb::TableError),
    #[error("提交错误：{0}")]
    Commit(#[from] redb::CommitError),
    #[error("存储错误：{0}")]
    Storage(#[from] redb::StorageError),
    #[error("SQLite 错误：{0}")]
    Sqlite(#[from] sea_orm::DbErr),
    #[error("文件系统错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误：{0}")]
    Encode(#[from] toml::ser::Error),
    #[error("反序列化错误：{0}")]
    Decode(#[from] toml::de::Error),
    #[error("数据格式错误：{0}")]
    InvalidData(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::{
        AgentSource, AuthProfile, CommandHistoryId, CommandHistoryItem, Host, HostId,
        HostNetworkSelection, ImageSource, RecentConnection, TunnelKind, TunnelRule,
    };
    use uuid::Uuid;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("smagicalssh-{name}-{}.redb", Uuid::new_v4()))
    }

    fn sample_host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            icon_key: "server".to_owned(),
            tags: vec!["prod".to_owned(), "linux".to_owned()],
            address: "prod.example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                source: AgentSource::Auto,
                key_hint: Some("id_ed25519".to_owned()),
            },
            network: HostNetworkSelection::default(),
            proxies: Vec::new(),
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    fn sample_tunnel_rule() -> TunnelRule {
        TunnelRule {
            name: "dynamic-proxy".to_owned(),
            kind: TunnelKind::Dynamic,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 1080,
            target_host: String::new(),
            target_port: 0,
            auto_start: false,
            exit_on_failure: false,
        }
    }

    #[test]
    fn missing_database_loads_empty_storage() {
        let path = temp_db_path("missing");
        let store = RedbStorage::new(path);

        let storage = store.load().expect("缺失数据库应该返回空存储");

        assert_eq!(storage, StorageManager::default());
    }

    #[test]
    fn storage_snapshot_round_trips_through_redb() {
        let path = temp_db_path("roundtrip");
        let store = RedbStorage::new(&path);
        let mut storage = StorageManager::default();
        storage.app_config.theme.name = "Solarized Dark".to_owned();
        storage.app_config.workspace.host_list_mode = smagical_config::HostListModePreference::Card;
        storage.app_config.background.enabled = true;
        storage.app_config.background.sources =
            vec![ImageSource::LocalPath("wallpapers/a.jpg".to_owned())];
        let host = sample_host();
        let host_id = host.id;
        storage.upsert_host(host);
        storage.upsert_tunnel_rule(sample_tunnel_rule());
        storage.record_recent_connection(RecentConnection {
            host_id,
            label: "production".to_owned(),
            connected_at_unix_secs: 1_700_000_000,
        });
        storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command: "uptime".to_owned(),
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: 1_700_000_001,
            duration_ms: None,
        });

        store.save(&storage).expect("存储快照应该可以写入 redb");
        let loaded = store.load().expect("存储快照应该可以从 redb 读取");

        assert_eq!(loaded, storage);
        assert_eq!(
            loaded.app_config.workspace.host_list_mode,
            smagical_config::HostListModePreference::Card
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn loaded_snapshot_reapplies_storage_invariants() {
        let path = temp_db_path("normalize");
        let store = RedbStorage::new(&path);
        let host = sample_host();
        let host_id = host.id;
        let mut raw = StorageSnapshot {
            hosts: vec![host],
            ..StorageSnapshot::default()
        };

        for index in 0..(DEFAULT_COMMAND_HISTORY_LIMIT + 3) {
            raw.command_history.push(CommandHistoryItem {
                id: CommandHistoryId(Uuid::new_v4()),
                host_id: Some(host_id),
                command: format!("cmd-{index}"),
                working_directory: None,
                exit_code: Some(0),
                started_at_unix_secs: index as u64,
                duration_ms: Some(1),
            });
        }

        let mut spaced_rule = sample_tunnel_rule();
        spaced_rule.name = " dynamic-proxy ".to_owned();
        spaced_rule.bind_host = " 127.0.0.1 ".to_owned();
        raw.tunnel_rules.push(sample_tunnel_rule());
        raw.tunnel_rules.push(spaced_rule);

        let encoded = toml::to_string(&raw).expect("测试快照应该可以序列化");
        let db = open_or_create_database(&path).expect("测试数据库应该可以创建");
        let write_txn = db.begin_write().expect("测试事务应该可以创建");
        {
            let mut table = write_txn
                .open_table(SNAPSHOT_TABLE)
                .expect("测试表应该可以打开");
            table
                .insert(SNAPSHOT_KEY, encoded.as_bytes())
                .expect("测试快照应该可以写入");
        }
        write_txn.commit().expect("测试事务应该可以提交");
        drop(db);

        let loaded = store.load().expect("存储快照应该可以从 redb 读取");

        assert_eq!(
            loaded.command_history_count(),
            DEFAULT_COMMAND_HISTORY_LIMIT
        );
        assert_eq!(loaded.command_history[0].command, "cmd-3");
        assert_eq!(loaded.tunnel_rule_count(), 1);
        let rule = loaded
            .tunnel_rule_by_name("dynamic-proxy")
            .expect("加载后应该按规范化名称查到隧道规则");
        assert_eq!(rule.bind_host, "127.0.0.1");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn save_creates_parent_directories() {
        let path = std::env::temp_dir()
            .join(format!("smagicalssh-nested-{}", Uuid::new_v4()))
            .join("state.redb");
        let store = RedbStorage::new(&path);
        let mut storage = StorageManager::default();
        storage.upsert_host(sample_host());

        store.save(&storage).expect("保存时应该自动创建父目录");

        assert!(path.exists());
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
