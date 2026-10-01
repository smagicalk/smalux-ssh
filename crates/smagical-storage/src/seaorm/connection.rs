//! SeaORM 统一连接池管理与自适应 DDL Schema 初始化。

use std::path::PathBuf;
use anyhow::{Context, Result};
use base64::Engine;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Schema,
};
use tracing::info;

use crate::crypto::{CryptoService, VaultManager};
use crate::entities::*;

/// 获取系统标准数据持久化文件路径 (Windows: %APPDATA%/smalux-ssh/data.db)
pub fn get_default_sqlite_path() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("dev", "smagical", "smalux-ssh") {
        let data_dir = dirs.data_dir();
        let _ = std::fs::create_dir_all(data_dir);
        data_dir.join("data.db")
    } else {
        PathBuf::from("data.db")
    }
}

/// 构造标准 SQLite 连接 URL (例如 "sqlite://C:/Users/.../data.db?mode=rwc")
pub fn get_default_sqlite_url() -> String {
    let path = get_default_sqlite_path();
    let path_str = path.to_string_lossy().replace('\\', "/");
    format!("sqlite://{}?mode=rwc", path_str)
}

/// 统一创建数据库连接池并确保完成 Schema 自动初始化
pub async fn establish_connection(db_url: &str) -> Result<DatabaseConnection> {
    tracing::debug!(target: "smagical_storage::seaorm", "正在连接存储数据库: [{}]", db_url);
    let mut opt = ConnectOptions::new(db_url);
    opt.sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug);
    let db = Database::connect(opt)
        .await
        .with_context(|| format!("连接数据库失败: {}", db_url))?;

    init_schema(&db).await.context("初始化数据库 DDL 结构失败")?;
    Ok(db)
}

/// 自动根据 Entity 契约建表 (自适应 SQLite / PostgreSQL / MySQL)
pub async fn init_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    macro_rules! create_table_if_not_exists {
        ($entity:expr) => {{
            let mut stmt = schema.create_table_from_entity($entity);
            stmt.if_not_exists();
            let statement = backend.build(&stmt);
            db.execute(statement).await?;
        }};
    }

    create_table_if_not_exists!(VaultSecurity);
    create_table_if_not_exists!(SystemMeta);
    create_table_if_not_exists!(Host);
    create_table_if_not_exists!(Group);
    create_table_if_not_exists!(Credential);
    create_table_if_not_exists!(History);
    create_table_if_not_exists!(HistorySnapshot);
    create_table_if_not_exists!(Snippet);
    create_table_if_not_exists!(SnippetGroup);
    create_table_if_not_exists!(Tunnel);
    create_table_if_not_exists!(AppConfig);
    create_table_if_not_exists!(BackupTask);
    create_table_if_not_exists!(BackupSnapshot);

    info!(target: "smagical_storage::seaorm", "数据库 Schema 全部数据表校验与初始化就绪");
    Ok(())
}

/// 初始化或挂载安全保险库 (首次启动自动生成默认机器级安全金丝雀，随时可被用户主密码接管)
pub async fn bootstrap_vault(db: &DatabaseConnection) -> Result<VaultManager> {
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let existing = VaultSecurity::find_by_id("master").one(db).await?;

    if let Some(record) = existing {
        // 已有保险库：当前处于默认/无主密码状态或等待解锁
        // 解密出 DEK (如果使用默认密码或无密码模式)
        let salt_bytes = base64::engine::general_purpose::STANDARD.decode(&record.kdf_salt)?;
        
        // 尝试使用系统默认/硬件固定密码进行初始尝试派生
        let default_master_key = CryptoService::derive_master_key(
            "smalux-default-device-key",
            &salt_bytes,
            record.kdf_memory_kib as u32,
            record.kdf_iterations as u32,
            record.kdf_parallelism as u32,
        )?;

        // 验证是否使用的是默认密码
        if CryptoService::verify_canary(&default_master_key, &record.canary_ciphertext) {
            let (_, dek_bytes) = CryptoService::decrypt(&default_master_key, &record.encrypted_dek)?;
            let mut dek = [0u8; 32];
            dek.copy_from_slice(&dek_bytes[..32]);
            return Ok(VaultManager::new_with_dek(dek, record.dek_version as u32));
        }

        // 否则为用户自定义主密码，初始状态处于锁定，需通过 UI 密码弹窗调用 unlock
        Ok(VaultManager::new_uninitialized())
    } else {
        // 首次初始化全新保险库：生成 32 字节 Salt 与全新 DEK
        let salt = CryptoService::generate_salt();
        let dek = CryptoService::generate_dek();
        let now = chrono::Utc::now().timestamp();

        let salt_b64 = base64::engine::general_purpose::STANDARD.encode(salt);
        let default_master_key = CryptoService::derive_master_key(
            "smalux-default-device-key",
            &salt,
            crate::crypto::DEFAULT_ARGON2_MEMORY_KIB,
            crate::crypto::DEFAULT_ARGON2_ITERATIONS,
            crate::crypto::DEFAULT_ARGON2_PARALLELISM,
        )?;

        let canary = CryptoService::create_canary(&default_master_key, 1)?;
        let encrypted_dek = CryptoService::encrypt(&default_master_key, 1, &dek)?;

        let new_model = vault_security::ActiveModel {
            id: Set("master".to_string()),
            kdf_algorithm: Set("argon2id".to_string()),
            kdf_salt: Set(salt_b64),
            kdf_iterations: Set(crate::crypto::DEFAULT_ARGON2_ITERATIONS as i32),
            kdf_memory_kib: Set(crate::crypto::DEFAULT_ARGON2_MEMORY_KIB as i32),
            kdf_parallelism: Set(crate::crypto::DEFAULT_ARGON2_PARALLELISM as i32),
            canary_ciphertext: Set(canary),
            encrypted_dek: Set(encrypted_dek),
            dek_version: Set(1),
            password_hint: Set(String::new()),
            last_rotated_at: Set(now),
            updated_at: Set(now),
        };

        new_model.insert(db).await?;
        info!(target: "smagical_storage::seaorm", "首次初始化安全保险库成功 (DEK v1 已激活)");

        Ok(VaultManager::new_with_dek(dek, 1))
    }
}
