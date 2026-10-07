//! 基于 SeaORM 的通用持久化存储聚合门面实现 (SeaOrmStorage)。

pub mod connection;
pub mod host_repo;
pub mod group_repo;
pub mod credential_repo;
pub mod history_repo;
pub mod snippet_repo;
pub mod tunnel_repo;
pub mod config_repo;
pub mod backup_repo;
pub mod real_seed;

pub use connection::{establish_connection, get_default_sqlite_path, get_default_sqlite_url, init_schema};
pub use host_repo::SeaOrmHostRepository;
pub use group_repo::SeaOrmGroupRepository;
pub use credential_repo::SeaOrmCredentialRepository;
pub use history_repo::SeaOrmHistoryRepository;
pub use snippet_repo::SeaOrmSnippetRepository;
pub use tunnel_repo::SeaOrmTunnelRepository;
pub use config_repo::SeaOrmConfigRepository;
pub use backup_repo::{SeaOrmBackupSnapshotRepository, SeaOrmBackupTaskRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::Engine;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set, TransactionTrait};
use smagical_core::storage::{
    AppStorage, BackupSnapshotRepository, BackupTaskRepository, ConfigRepository,
    CredentialRepository, GroupRepository, HistoryRepository, HostRepository, SnippetRepository,
    StorageError, StorageResult, TunnelRepository,
};
use tracing::info;

use crate::crypto::{CryptoService, VaultManager};
use crate::entities::*;

/// SeaORM 聚合持久化存储驱动 (支持 SQLite / PostgreSQL / MySQL)
#[derive(Clone)]
pub struct SeaOrmStorage {
    db: DatabaseConnection,
    vault: VaultManager,
    hosts_repo: SeaOrmHostRepository,
    groups_repo: SeaOrmGroupRepository,
    credentials_repo: SeaOrmCredentialRepository,
    history_repo: SeaOrmHistoryRepository,
    snippets_repo: SeaOrmSnippetRepository,
    tunnels_repo: SeaOrmTunnelRepository,
    config_repo: SeaOrmConfigRepository,
    backup_tasks_repo: SeaOrmBackupTaskRepository,
    backup_snapshots_repo: SeaOrmBackupSnapshotRepository,
}

impl SeaOrmStorage {
    /// 打开本地默认的 SQLite 数据库文件并初始化 Schema 与安全保险库
    pub async fn open_default() -> Result<Self> {
        let url = get_default_sqlite_url();
        Self::open_url(&url).await
    }

    /// 根据统一连接 URL (如 "sqlite://...", "postgres://...", "mysql://...") 连接数据库
    pub async fn open_url(url: &str) -> Result<Self> {
        let db = establish_connection(url).await?;
        let vault = connection::bootstrap_vault(&db).await?;

        let hosts_repo = SeaOrmHostRepository::new(db.clone(), vault.clone());
        let groups_repo = SeaOrmGroupRepository::new(db.clone());
        let credentials_repo = SeaOrmCredentialRepository::new(db.clone(), vault.clone());
        let history_repo = SeaOrmHistoryRepository::new(db.clone());
        let snippets_repo = SeaOrmSnippetRepository::new(db.clone());
        let tunnels_repo = SeaOrmTunnelRepository::new(db.clone());
        let config_repo = SeaOrmConfigRepository::new(db.clone());
        let backup_tasks_repo = SeaOrmBackupTaskRepository::new(db.clone(), vault.clone());
        let backup_snapshots_repo = SeaOrmBackupSnapshotRepository::new(db.clone());

        let storage = Self {
            db,
            vault,
            hosts_repo,
            groups_repo,
            credentials_repo,
            history_repo,
            snippets_repo,
            tunnels_repo,
            config_repo,
            backup_tasks_repo,
            backup_snapshots_repo,
        };

        // 自动完成真实资产嗅探与生产级默认配置初始化 (杜绝 Mock 假数据侵入生产环境)
        let _ = storage.ensure_real_user_data().await;

        Ok(storage)
    }

    /// 获取底层 SeaORM 数据库连接句柄
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    /// 获取底层安全保险库管理器引用
    pub fn vault(&self) -> &VaultManager {
        &self.vault
    }

    /// 确保物理数据库完成真实数据与本地资产初始化，并在检测到旧版 Mock 假数据时自动迁移/清理
    pub async fn ensure_real_user_data(&self) -> Result<bool> {
        // 1. 检查是否存在历史遗留的假数据 (Mock Seed Data)
        // 假数据特征: 存在 "auth-gateway-edge", "prod-server-01", "grp-prod", 或 db_initialized == "true"
        let is_legacy_seed = match SystemMeta::find_by_id("db_initialized").one(&self.db).await {
            Ok(Some(meta)) => meta.meta_value == "true",
            _ => false,
        };

        let has_mock_hosts = Host::find_by_id("auth-gateway-edge").one(&self.db).await.ok().flatten().is_some()
            || Host::find_by_id("prod-server-01").one(&self.db).await.ok().flatten().is_some()
            || Host::find_by_id("1").one(&self.db).await.ok().flatten().is_some()
            || Group::find_by_id("grp-prod").one(&self.db).await.ok().flatten().is_some();

        if is_legacy_seed || has_mock_hosts {
            info!(target: "smagical_storage::seaorm", "检测到历史遗留 Mock 假数据，正在执行自动清理与真实资产迁移...");
            // 清理旧版假数据
            let legacy_host_ids = ["1", "2", "host-k8s-w1", "3", "host-db-s1", "4", "5", "6", "7", "host-staging", "prod-server-01", "auth-gateway-edge", "dev-db-01", "jump-server-01", "stage-k8s-master"];
            for hid in &legacy_host_ids {
                let _ = self.hosts_repo.delete(hid).await;
            }
            let legacy_group_ids = ["grp-prod", "grp-k8s", "grp-db", "grp-edge", "grp-ai", "grp-dr", "grp-dev", "grp-infra"];
            for gid in &legacy_group_ids {
                let _ = self.groups_repo.delete(gid).await;
            }
            let legacy_cred_ids = ["cred-prod-ed25519", "cred-bastion-pwd", "cred-1pwd-agent", "cred-dev-rsa", "cred-openssh-agent", "cred-bitwarden-agent", "cred-dev-password", "cred-jump-agent"];
            for cid in &legacy_cred_ids {
                let _ = self.credentials_repo.delete(cid).await;
            }
            let legacy_tunnel_ids = ["tun-pg-local", "tun-k8s-dash"];
            for tid in &legacy_tunnel_ids {
                let _ = self.tunnels_repo.delete(tid).await;
            }
            let legacy_snip_groups = ["sg-docker", "sg-sys", "sg-k8s"];
            for sgid in &legacy_snip_groups {
                let _ = self.snippets_repo.delete_group(sgid).await;
            }
            let legacy_snips = ["snip-docker-ps", "snip-sys-load", "snip-k8s-pods", "snip-curl-health"];
            for sid in &legacy_snips {
                let _ = self.snippets_repo.delete(sid).await;
            }
        }

        // 2. 检查系统元数据标识
        if let Ok(Some(meta)) = SystemMeta::find_by_id("db_initialized").one(&self.db).await {
            // 如果用户显式清空或用户显式重置为演示数据，则遵循用户意图
            if meta.meta_value == "cleared" || meta.meta_value == "demo" {
                return Ok(false);
            }
            if meta.meta_value == "real_data" {
                let host_count = Host::find().count(&self.db).await.unwrap_or(0);
                let group_count = Group::find().count(&self.db).await.unwrap_or(0);
                if host_count > 0 || group_count > 0 {
                    return Ok(false);
                }
            }
        }

        // 3. 执行真实资产与默认生产配置初始化
        self.seed_real_default_data().await
    }

    /// 初始化真实用户数据环境：
    /// 1. 创建默认基础分组 ("我的主机")
    /// 2. 扫描本机 ~/.ssh (known_hosts 与 config)，若发现资产则建立 "已知主机 (Known Hosts)" 并自动收敛入库
    /// 3. 导入生产级 Linux 运维命令片段集 (df, free, top, ports 等)
    /// 4. 写入系统元数据 db_initialized = "real_data"
    pub async fn seed_real_default_data(&self) -> Result<bool> {
        info!(target: "smagical_storage::seaorm", "正在初始化真实数据环境与探测本机 OpenSSH 资产...");

        // 1. 创建默认根分组: 我的主机
        let default_group = smagical_core::domain::group::GroupRecord::root("grp-default", "我的主机");
        let _ = self.groups_repo.save(&default_group).await;

        // 2. 扫描本地 ~/.ssh
        let (config_hosts, known_hosts) = real_seed::discover_local_ssh_hosts(
            Some("grp-known".to_string()),
            Some("grp-default".to_string()),
        );

        // 如果在 known_hosts 中发现了已知真实主机，创建 "已知主机" 分组
        if !known_hosts.is_empty() {
            let mut known_group = smagical_core::domain::group::GroupRecord::root("grp-known", "已知主机 (Known Hosts)");
            known_group.sort_order = 1;
            let _ = self.groups_repo.save(&known_group).await;

            for h in known_hosts {
                let _ = self.hosts_repo.save(&h).await;
            }
        }

        // 保存来自 ~/.ssh/config 的主机
        for h in config_hosts {
            let _ = self.hosts_repo.save(&h).await;
        }

        // 3. 导入生产级常用命令片段
        let (snip_groups, snips) = real_seed::generate_real_snippets();
        for sg in snip_groups {
            let _ = self.snippets_repo.save_group(&sg).await;
        }
        for s in snips {
            let _ = self.snippets_repo.save(&s).await;
        }

        // 4. 更新元数据标记为 real_data
        let now = chrono::Utc::now().timestamp();
        match SystemMeta::find_by_id("db_initialized").one(&self.db).await {
            Ok(Some(existing)) => {
                let mut act: system_meta::ActiveModel = existing.into();
                act.meta_value = Set("real_data".to_string());
                act.updated_at = Set(now);
                let _ = act.update(&self.db).await;
            }
            _ => {
                let act = system_meta::ActiveModel {
                    meta_key: Set("db_initialized".to_string()),
                    meta_value: Set("real_data".to_string()),
                    updated_at: Set(now),
                };
                let _ = act.insert(&self.db).await;
            }
        }

        info!(target: "smagical_storage::seaorm", "真实数据环境初始化完成 (已完全排除 Mock 演示数据)");
        Ok(true)
    }

    /// 兼容旧版调用的别名方法
    pub async fn seed_if_empty(&self) -> Result<bool> {
        self.ensure_real_user_data().await
    }

    /// 清空所有预设演示种子数据（主机、分组、密钥、代码片段与网络隧道规则）
    pub async fn clear_demo_data(&self) -> Result<()> {
        info!(target: "smagical_storage::seaorm", "正在清空物理数据库中的所有数据并标记为纯净库...");
        let now = chrono::Utc::now().timestamp();
        match SystemMeta::find_by_id("db_initialized").one(&self.db).await {
            Ok(Some(existing)) => {
                let mut act: system_meta::ActiveModel = existing.into();
                act.meta_value = Set("cleared".to_string());
                act.updated_at = Set(now);
                let _ = act.update(&self.db).await;
            }
            _ => {
                let act = system_meta::ActiveModel {
                    meta_key: Set("db_initialized".to_string()),
                    meta_value: Set("cleared".to_string()),
                    updated_at: Set(now),
                };
                let _ = act.insert(&self.db).await;
            }
        }

        if let Ok(hosts) = self.hosts_repo.list_all().await {
            for h in hosts { let _ = self.hosts_repo.delete(&h.id).await; }
        }
        if let Ok(groups) = self.groups_repo.list_all().await {
            for g in groups { let _ = self.groups_repo.delete(&g.id).await; }
        }
        if let Ok(creds) = self.credentials_repo.list_all().await {
            for c in creds { let _ = self.credentials_repo.delete(&c.id).await; }
        }
        if let Ok(snippets) = self.snippets_repo.list_all().await {
            for s in snippets { let _ = self.snippets_repo.delete(&s.id).await; }
        }
        if let Ok(s_groups) = self.snippets_repo.list_groups().await {
            for sg in s_groups { let _ = self.snippets_repo.delete_group(&sg.id).await; }
        }
        if let Ok(tunnels) = self.tunnels_repo.list_all().await {
            for t in tunnels { let _ = self.tunnels_repo.delete(&t.id).await; }
        }
        info!(target: "smagical_storage::seaorm", "物理数据库已全部清空，进入纯净可用状态");
        Ok(())
    }

    /// 重新恢复预设演示种子数据
    pub async fn reset_demo_data(&self) -> Result<()> {
        let _ = self.clear_demo_data().await;
        let seed = crate::mock::seed_data::generate_seed_data();
        for g in seed.groups { let _ = self.groups_repo.save(&g).await; }
        for h in seed.hosts { let _ = self.hosts_repo.save(&h).await; }
        for sg in seed.snippet_groups { let _ = self.snippets_repo.save_group(&sg).await; }
        for s in seed.snippets { let _ = self.snippets_repo.save(&s).await; }
        for t in seed.tunnels { let _ = self.tunnels_repo.save(&t).await; }
        for c in seed.credentials { let _ = self.credentials_repo.save(&c).await; }
        for h in seed.history { let _ = self.history_repo.save(&h).await; }
        for (hid, snap) in seed.snapshots {
            let _ = self.history_repo.save_snapshot(&hid, &snap, 500).await;
        }
        let now = chrono::Utc::now().timestamp();
        match SystemMeta::find_by_id("db_initialized").one(&self.db).await {
            Ok(Some(existing)) => {
                let mut act: system_meta::ActiveModel = existing.into();
                act.meta_value = Set("demo".to_string());
                act.updated_at = Set(now);
                let _ = act.update(&self.db).await;
            }
            _ => {
                let act = system_meta::ActiveModel {
                    meta_key: Set("db_initialized".to_string()),
                    meta_value: Set("demo".to_string()),
                    updated_at: Set(now),
                };
                let _ = act.insert(&self.db).await;
            }
        }
        info!(target: "smagical_storage::seaorm", "预设演示种子数据已重新导入完成");
        Ok(())
    }

    /// 查询保险库当前是否处于解锁就绪状态
    pub fn is_vault_unlocked(&self) -> bool {
        self.vault.is_unlocked()
    }

    /// 查询当前是否设置了自定义主密码 (而非默认设备种子)
    pub async fn has_custom_master_password(&self) -> StorageResult<bool> {
        let existing = VaultSecurity::find_by_id("master")
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match existing {
            Some(record) => {
                let salt_bytes = base64::engine::general_purpose::STANDARD
                    .decode(&record.kdf_salt)
                    .map_err(|e| StorageError::Backend(e.to_string()))?;

                let default_master_key = CryptoService::derive_master_key(
                    "smalux-default-device-key",
                    &salt_bytes,
                    record.kdf_memory_kib as u32,
                    record.kdf_iterations as u32,
                    record.kdf_parallelism as u32,
                ).map_err(|e| StorageError::Backend(e.to_string()))?;

                // 若默认密码验证通过金丝雀，说明用户尚未设置自定义主密码
                let is_default = CryptoService::verify_canary(&default_master_key, &record.canary_ciphertext);
                Ok(!is_default)
            }
            None => Ok(false),
        }
    }

    /// 使用主密码解锁保险库并挂载 DEK (返回 true 表示解锁成功，false 表示密码错误)
    pub async fn unlock_vault(&self, password: &str) -> StorageResult<bool> {
        if self.vault.is_unlocked() {
            return Ok(true);
        }

        let existing = VaultSecurity::find_by_id("master")
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .ok_or_else(|| StorageError::NotFound("安全保险库未就绪".into()))?;

        let salt_bytes = base64::engine::general_purpose::STANDARD
            .decode(&existing.kdf_salt)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let master_key = CryptoService::derive_master_key(
            password,
            &salt_bytes,
            existing.kdf_memory_kib as u32,
            existing.kdf_iterations as u32,
            existing.kdf_parallelism as u32,
        ).map_err(|e| StorageError::Backend(e.to_string()))?;

        if !CryptoService::verify_canary(&master_key, &existing.canary_ciphertext) {
            return Ok(false);
        }

        let (_, dek_bytes) = CryptoService::decrypt(&master_key, &existing.encrypted_dek)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if dek_bytes.len() < 32 {
            return Err(StorageError::Backend("解密出的 DEK 密钥长度不足 32 字节".into()));
        }

        let mut dek = [0u8; 32];
        dek.copy_from_slice(&dek_bytes[..32]);

        self.vault.unlock_with_dek(dek, existing.dek_version as u32);
        info!(target: "smagical_storage::seaorm", "保险库通过主密码成功解锁 (DEK v{} 已挂载)", existing.dek_version);
        Ok(true)
    }

    /// 用户修改或设置安全主密码 (信封加密: 仅重新加密 DEK，无需重新加密成千上万台主机数据，耗时 0.01s)
    pub async fn change_master_password(
        &self,
        old_password_opt: Option<&str>,
        new_password: &str,
        hint: &str,
    ) -> StorageResult<()> {
        let existing = VaultSecurity::find_by_id("master")
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .ok_or_else(|| StorageError::NotFound("安全保险库记录未找到".into()))?;

        let salt_bytes = base64::engine::general_purpose::STANDARD
            .decode(&existing.kdf_salt)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        // 1. 验证旧密码有效性
        let old_pass = old_password_opt.unwrap_or("smalux-default-device-key");
        let old_master_key = CryptoService::derive_master_key(
            old_pass,
            &salt_bytes,
            existing.kdf_memory_kib as u32,
            existing.kdf_iterations as u32,
            existing.kdf_parallelism as u32,
        ).map_err(|e| StorageError::Backend(e.to_string()))?;

        if !CryptoService::verify_canary(&old_master_key, &existing.canary_ciphertext) {
            return Err(StorageError::Backend("原安全密码验证失败，拒绝修改密码".into()));
        }

        // 2. 解密出当前生效的 DEK
        let (_, dek_bytes) = CryptoService::decrypt(&old_master_key, &existing.encrypted_dek)
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        if dek_bytes.len() < 32 {
            return Err(StorageError::Backend("解密出的 DEK 密钥长度不足 32 字节".into()));
        }
        let mut dek = [0u8; 32];
        dek.copy_from_slice(&dek_bytes[..32]);

        // 3. 为新密码生成全新随机 Salt
        let new_salt = CryptoService::generate_salt();
        let new_salt_b64 = base64::engine::general_purpose::STANDARD.encode(new_salt);

        // 4. 派生全新 MasterKey 并重新加密 DEK 与金丝雀
        let new_master_key = CryptoService::derive_master_key(
            new_password,
            &new_salt,
            existing.kdf_memory_kib as u32,
            existing.kdf_iterations as u32,
            existing.kdf_parallelism as u32,
        ).map_err(|e| StorageError::Backend(e.to_string()))?;

        let new_canary = CryptoService::create_canary(&new_master_key, existing.dek_version as u32)
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        let new_encrypted_dek = CryptoService::encrypt(&new_master_key, existing.dek_version as u32, &dek)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let now = chrono::Utc::now().timestamp();
        let dek_ver = existing.dek_version as u32;
        let mut active: vault_security::ActiveModel = existing.into();
        active.kdf_salt = Set(new_salt_b64);
        active.canary_ciphertext = Set(new_canary);
        active.encrypted_dek = Set(new_encrypted_dek);
        active.password_hint = Set(hint.to_string());
        active.updated_at = Set(now);

        active.update(&self.db).await
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        self.vault.unlock_with_dek(dek, dek_ver);

        info!(target: "smagical_storage::seaorm", "安全主密码已更新 (信封重新封装完成)");
        Ok(())
    }

    /// 移除自定义主密码，恢复为开箱即用的默认无密码模式 (信封重封: 换用默认设备种子加密 DEK)
    pub async fn remove_master_password(&self, current_password: &str) -> StorageResult<()> {
        let existing = VaultSecurity::find_by_id("master")
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .ok_or_else(|| StorageError::NotFound("安全保险库未就绪".into()))?;

        let salt_bytes = base64::engine::general_purpose::STANDARD
            .decode(&existing.kdf_salt)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let master_key = CryptoService::derive_master_key(
            current_password,
            &salt_bytes,
            existing.kdf_memory_kib as u32,
            existing.kdf_iterations as u32,
            existing.kdf_parallelism as u32,
        ).map_err(|e| StorageError::Backend(e.to_string()))?;

        if !CryptoService::verify_canary(&master_key, &existing.canary_ciphertext) {
            return Err(StorageError::Backend("原安全主密码验证失败，拒绝移除".into()));
        }

        let (_, dek_bytes) = CryptoService::decrypt(&master_key, &existing.encrypted_dek)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if dek_bytes.len() < 32 {
            return Err(StorageError::Backend("解密出的 DEK 密钥长度不足 32 字节".into()));
        }

        let mut dek = [0u8; 32];
        dek.copy_from_slice(&dek_bytes[..32]);

        // 生成全新 Salt，使用默认设备种子重新封装原 DEK
        let new_salt = CryptoService::generate_salt();
        let new_salt_b64 = base64::engine::general_purpose::STANDARD.encode(new_salt);

        let default_master_key = CryptoService::derive_master_key(
            "smalux-default-device-key",
            &new_salt,
            existing.kdf_memory_kib as u32,
            existing.kdf_iterations as u32,
            existing.kdf_parallelism as u32,
        ).map_err(|e| StorageError::Backend(e.to_string()))?;

        let dek_ver = existing.dek_version as u32;
        let new_canary = CryptoService::create_canary(&default_master_key, dek_ver)
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        let new_encrypted_dek = CryptoService::encrypt(&default_master_key, dek_ver, &dek)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let now = chrono::Utc::now().timestamp();
        let mut active: vault_security::ActiveModel = existing.into();
        active.kdf_salt = Set(new_salt_b64);
        active.canary_ciphertext = Set(new_canary);
        active.encrypted_dek = Set(new_encrypted_dek);
        active.password_hint = Set(String::new());
        active.updated_at = Set(now);

        active.update(&self.db).await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        self.vault.unlock_with_dek(dek, dek_ver);
        info!(target: "smagical_storage::seaorm", "已成功移除自定义主密码，恢复为默认无密码存储模式");
        Ok(())
    }

    /// 全量数据密钥轮换 (全量重加密翻新: 在 ACID 事务保护下生成新 DEK 并重写所有敏感密文)
    pub async fn rotate_dek(&self, master_password: &str) -> Result<u32> {
        let existing = VaultSecurity::find_by_id("master")
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow!("安全保险库未就绪"))?;

        let salt_bytes = base64::engine::general_purpose::STANDARD.decode(&existing.kdf_salt)?;
        let master_key = CryptoService::derive_master_key(
            master_password,
            &salt_bytes,
            existing.kdf_memory_kib as u32,
            existing.kdf_iterations as u32,
            existing.kdf_parallelism as u32,
        )?;

        if !CryptoService::verify_canary(&master_key, &existing.canary_ciphertext) {
            return Err(anyhow!("安全主密码验证失败，拒绝执行全量重加密轮换"));
        }

        let (_, old_dek_bytes) = CryptoService::decrypt(&master_key, &existing.encrypted_dek)?;
        let mut old_dek = [0u8; 32];
        old_dek.copy_from_slice(&old_dek_bytes[..32]);

        // 生成全新 DEK (版本 +1)
        let new_dek = CryptoService::generate_dek();
        let new_version = (existing.dek_version + 1) as u32;

        info!(target: "smagical_storage::seaorm", "正在启动全量数据密钥轮换 (DEK v{} -> v{})...", existing.dek_version, new_version);

        // 开启数据库事务保证原子安全性
        let txn = self.db.begin().await?;

        // 1. 遍历并重新加密 credentials
        let all_creds = Credential::find().all(&txn).await?;
        for c in all_creds {
            let mut active: credential::ActiveModel = c.clone().into();
            if !c.secret_data_enc.is_empty() {
                if let Ok((_, raw)) = CryptoService::decrypt(&old_dek, &c.secret_data_enc) {
                    let new_enc = CryptoService::encrypt(&new_dek, new_version, &raw)?;
                    active.secret_data_enc = Set(new_enc);
                }
            }
            if let Some(ref pass) = c.passphrase_enc {
                if !pass.is_empty() {
                    if let Ok((_, raw)) = CryptoService::decrypt(&old_dek, pass) {
                        let new_enc = CryptoService::encrypt(&new_dek, new_version, &raw)?;
                        active.passphrase_enc = Set(Some(new_enc));
                    }
                }
            }
            active.update(&txn).await?;
        }

        // 2. 遍历并重新加密 hosts
        let all_hosts = Host::find().all(&txn).await?;
        for h in all_hosts {
            let mut active: host::ActiveModel = h.clone().into();
            if let Some(ref p) = h.password_enc {
                if !p.is_empty() {
                    if let Ok((_, raw)) = CryptoService::decrypt(&old_dek, p) {
                        let new_enc = CryptoService::encrypt(&new_dek, new_version, &raw)?;
                        active.password_enc = Set(Some(new_enc));
                    }
                }
            }
            if let Some(ref k) = h.key_data_enc {
                if !k.is_empty() {
                    if let Ok((_, raw)) = CryptoService::decrypt(&old_dek, k) {
                        let new_enc = CryptoService::encrypt(&new_dek, new_version, &raw)?;
                        active.key_data_enc = Set(Some(new_enc));
                    }
                }
            }
            if let Some(ref kp) = h.key_passphrase_enc {
                if !kp.is_empty() {
                    if let Ok((_, raw)) = CryptoService::decrypt(&old_dek, kp) {
                        let new_enc = CryptoService::encrypt(&new_dek, new_version, &raw)?;
                        active.key_passphrase_enc = Set(Some(new_enc));
                    }
                }
            }
            if let Some(ref pp) = h.proxy_password_enc {
                if !pp.is_empty() {
                    if let Ok((_, raw)) = CryptoService::decrypt(&old_dek, pp) {
                        let new_enc = CryptoService::encrypt(&new_dek, new_version, &raw)?;
                        active.proxy_password_enc = Set(Some(new_enc));
                    }
                }
            }
            active.update(&txn).await?;
        }

        // 3. 更新 vault_security
        let new_encrypted_dek = CryptoService::encrypt(&master_key, new_version, &new_dek)?;
        let new_canary = CryptoService::create_canary(&master_key, new_version)?;
        let now = chrono::Utc::now().timestamp();

        let mut active_sec: vault_security::ActiveModel = existing.into();
        active_sec.encrypted_dek = Set(new_encrypted_dek);
        active_sec.canary_ciphertext = Set(new_canary);
        active_sec.dek_version = Set(new_version as i32);
        active_sec.last_rotated_at = Set(now);
        active_sec.updated_at = Set(now);
        active_sec.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 挂载新密钥进内存
        self.vault.unlock_with_dek(new_dek, new_version);

        info!(target: "smagical_storage::seaorm", "全量密钥轮换成功！当前活跃版本: DEK v{}", new_version);
        Ok(new_version)
    }
}

#[async_trait]
impl AppStorage for SeaOrmStorage {
    fn hosts(&self) -> &dyn HostRepository {
        &self.hosts_repo
    }

    fn groups(&self) -> &dyn GroupRepository {
        &self.groups_repo
    }

    fn history(&self) -> &dyn HistoryRepository {
        &self.history_repo
    }

    fn credentials(&self) -> &dyn CredentialRepository {
        &self.credentials_repo
    }

    fn snippets(&self) -> &dyn SnippetRepository {
        &self.snippets_repo
    }

    fn tunnels(&self) -> &dyn TunnelRepository {
        &self.tunnels_repo
    }

    fn config(&self) -> &dyn ConfigRepository {
        &self.config_repo
    }

    fn backup_tasks(&self) -> &dyn BackupTaskRepository {
        &self.backup_tasks_repo
    }

    fn backup_snapshots(&self) -> &dyn BackupSnapshotRepository {
        &self.backup_snapshots_repo
    }

    async fn reload(&self) -> StorageResult<()> {
        // SeaORM 每次查询直通底层连接池，天然实时
        Ok(())
    }

    async fn flush(&self) -> StorageResult<()> {
        // SeaORM 写入立即落盘事务提交，天然持久化
        Ok(())
    }

    fn is_vault_unlocked(&self) -> bool {
        self.is_vault_unlocked()
    }

    fn lock_vault(&self) {
        self.vault.lock();
    }

    async fn has_custom_master_password(&self) -> StorageResult<bool> {
        self.has_custom_master_password().await
    }

    async fn unlock_vault(&self, password: &str) -> StorageResult<bool> {
        self.unlock_vault(password).await
    }

    async fn change_master_password(
        &self,
        old_password_opt: Option<&str>,
        new_password: &str,
        hint: &str,
    ) -> StorageResult<()> {
        self.change_master_password(old_password_opt, new_password, hint).await
    }

    async fn remove_master_password(&self, current_password: &str) -> StorageResult<()> {
        self.remove_master_password(current_password).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::domain::host::{HostRecord, HostStatus};

    #[tokio::test]
    async fn test_seaorm_storage_full_lifecycle_and_encryption() {
        // 使用内存 SQLite 进行真实 SeaORM 测试
        let storage = SeaOrmStorage::open_url("sqlite::memory:")
            .await
            .expect("创建内存 SeaOrm 数据库应成功");

        // 1. 验证自动种子导入
        let hosts = storage.hosts().list_all().await.expect("读取主机应成功");
        assert!(!hosts.is_empty(), "初始种子主机列表不应为空");

        // 2. 新增一台带敏感密码的主机
        let new_host = HostRecord {
            id: "host-test-enc".to_string(),
            name: "Test Secure Server".to_string(),
            address: "192.168.10.50".to_string(),
            port: 2222,
            parent_group_id: None,
            credential_id: None,
            status: HostStatus::Online,
            ping_ms: 15,
            sort_order: 100,
            notes: "Encrypted test host".to_string(),
            auth_type: "password".to_string(),
            username: Some("admin".to_string()),
            password: Some("TopSecretP@ssword!".to_string()),
            key_data: Some("-----BEGIN PRIVATE KEY-----\nMIIEvg...".to_string()),
            key_passphrase: Some("Passphrase123".to_string()),
            ..Default::default()
        };
        storage.hosts().save(&new_host).await.expect("保存主机应成功");

        // 验证从数据库直接查原始 Model: 确认密码与私钥在数据库中是密文存盘的 (以 enc:v1: 开头)
        let raw_model = Host::find_by_id("host-test-enc")
            .one(storage.db())
            .await
            .unwrap()
            .unwrap();
        let pass_enc = raw_model.password_enc.unwrap();
        assert!(pass_enc.starts_with("enc:v1:"), "数据库落盘密码必须加密: {}", pass_enc);
        assert_ne!(pass_enc, "TopSecretP@ssword!");

        let key_enc = raw_model.key_data_enc.unwrap();
        assert!(key_enc.starts_with("enc:v1:"), "数据库落盘私钥必须加密: {}", key_enc);

        // 验证通过仓储层读取: 透明自动解密为原始明文
        let read_back = storage
            .hosts()
            .get_by_id("host-test-enc")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(read_back.password.as_deref(), Some("TopSecretP@ssword!"));
        assert_eq!(read_back.key_data.as_deref(), Some("-----BEGIN PRIVATE KEY-----\nMIIEvg..."));
        assert_eq!(read_back.key_passphrase.as_deref(), Some("Passphrase123"));

        // 3. 测试更改主密码 (信封重新封装)
        let change_res = storage
            .change_master_password(None, "NewUserMasterPassword#2026", "My secret hint")
            .await;
        assert!(change_res.is_ok(), "修改主密码应成功: {:?}", change_res.err());

        // 验证改完主密码后，资产仍可正常解密
        let read_after_pwd_change = storage
            .hosts()
            .get_by_id("host-test-enc")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(read_after_pwd_change.password.as_deref(), Some("TopSecretP@ssword!"));

        // 4. 测试全量重加密轮换 (DEK 轮换: v1 -> v2)
        let rot_res = storage.rotate_dek("NewUserMasterPassword#2026").await;
        assert!(rot_res.is_ok(), "全量 DEK 轮换应成功: {:?}", rot_res.err());
        assert_eq!(rot_res.unwrap(), 2, "轮换后版本应升至 2");

        // 验证数据库底层密文已全量翻新为 enc:v2: 开头
        let raw_v2_model = Host::find_by_id("host-test-enc")
            .one(storage.db())
            .await
            .unwrap()
            .unwrap();
        let pass_enc_v2 = raw_v2_model.password_enc.unwrap();
        assert!(pass_enc_v2.starts_with("enc:v2:"), "轮换后密码必须为 v2 密文: {}", pass_enc_v2);

        // 验证轮换后仓储层解密依然 100% 准确无损
        let read_after_rotate = storage
            .hosts()
            .get_by_id("host-test-enc")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(read_after_rotate.password.as_deref(), Some("TopSecretP@ssword!"));
        assert_eq!(read_after_rotate.key_data.as_deref(), Some("-----BEGIN PRIVATE KEY-----\nMIIEvg..."));
    }

    #[tokio::test]
    async fn test_seaorm_storage_physical_file_and_reopen() {
        let temp_dir = std::env::temp_dir().join(format!("smalux_test_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("test_physical.db");
        let path_str = db_path.to_string_lossy().replace('\\', "/");
        let url = format!("sqlite://{}?mode=rwc", path_str);

        // 第一次打开并初始化种子
        {
            let storage = SeaOrmStorage::open_url(&url).await.expect("创建物理 SQLite 文件应成功");
            let hosts = storage.hosts().list_all().await.expect("读取主机列表应成功");
            assert!(!hosts.is_empty(), "初始种子主机不应为空");
            assert!(db_path.exists(), "物理数据库文件应存在于磁盘");
        }

        // 第二次重新打开已存在的物理数据库文件
        {
            let storage2 = SeaOrmStorage::open_url(&url).await.expect("重新打开物理 SQLite 文件应成功");
            let hosts2 = storage2.hosts().list_all().await.expect("再次读取主机应成功");
            assert!(!hosts2.is_empty(), "重新打开后数据依然持久化存在");
        }

        // 清理临时文件
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_seaorm_master_password_and_unlock_lifecycle() {
        let storage = SeaOrmStorage::open_url("sqlite::memory:")
            .await
            .expect("创建内存数据库应成功");

        // 1. 初始状态：无自定义密码，自动解锁
        assert!(!storage.has_custom_master_password().await.unwrap(), "初始应无自定义主密码");
        assert!(storage.is_vault_unlocked(), "初始保险库应自动解锁就绪");

        // 2. 插入带密码的凭据记录
        let cred = smagical_core::domain::credential::CredentialRecord::new_password(
            "cred-test-pwd",
            "Server Root Key",
            "root",
            "UltraSecurePassword@2026",
            "Secret cred notes",
        );
        storage.credentials().save(&cred).await.unwrap();

        // 3. 用户设置主密码
        storage.change_master_password(None, "MasterKey!999", "remember me").await.unwrap();
        assert!(storage.has_custom_master_password().await.unwrap(), "设置主密码后应识别为自定义主密码");
        assert!(storage.is_vault_unlocked(), "设置密码后内存 DEK 仍有效保持解锁");

        // 4. 模拟冷启动或手动锁定保险库
        storage.vault().lock();
        assert!(!storage.is_vault_unlocked(), "锁定后保险库状态应为未解锁");

        // 5. 错误密码解锁尝试
        let wrong_unlock = storage.unlock_vault("WrongPassword").await.unwrap();
        assert!(!wrong_unlock, "错误密码应返回 false");
        assert!(!storage.is_vault_unlocked(), "错误密码解锁后应仍处于锁定状态");

        // 6. 正确密码解锁
        let correct_unlock = storage.unlock_vault("MasterKey!999").await.unwrap();
        assert!(correct_unlock, "正确密码应返回 true");
        assert!(storage.is_vault_unlocked(), "解锁成功后状态应变为 true");

        // 验证读取凭据解密正常
        let read_cred = storage.credentials().get_by_id("cred-test-pwd").await.unwrap().unwrap();
        assert_eq!(read_cred.secret_data.as_str(), "UltraSecurePassword@2026");

        // 7. 移除主密码 (回退开箱即用模式)
        let fail_remove = storage.remove_master_password("WrongPassword").await;
        assert!(fail_remove.is_err(), "错误密码移除主密码应失败");

        let ok_remove = storage.remove_master_password("MasterKey!999").await;
        assert!(ok_remove.is_ok(), "正确密码移除主密码应成功");

        // 8. 移除后状态检查
        assert!(!storage.has_custom_master_password().await.unwrap(), "移除后应恢复为非自定义主密码");
        assert!(storage.is_vault_unlocked(), "移除后应保持解锁");

        // 9. 再次验证冷启动默认密码无感解锁
        storage.vault().lock();
        assert!(!storage.is_vault_unlocked());
        let default_unlock = storage.unlock_vault("smalux-default-device-key").await.unwrap();
        assert!(default_unlock, "回退后可用默认设备种子解锁");
        let read_again = storage.credentials().get_by_id("cred-test-pwd").await.unwrap().unwrap();
        assert_eq!(read_again.secret_data.as_str(), "UltraSecurePassword@2026");
    }
}
