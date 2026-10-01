//! SeaORM 主机资产仓储实现 (SeaOrmHostRepository)。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use smagical_core::domain::host::{HostRecord, HostStatus};
use smagical_core::storage::{HostRepository, StorageError, StorageResult};

use crate::crypto::VaultManager;
use crate::entities::host;
use crate::entities::Host;

/// 基于 SeaORM 的主机资产持久化仓储
#[derive(Clone)]
pub struct SeaOrmHostRepository {
    db: DatabaseConnection,
    vault: VaultManager,
}

impl SeaOrmHostRepository {
    /// 构造新主机仓储实例
    pub fn new(db: DatabaseConnection, vault: VaultManager) -> Self {
        Self { db, vault }
    }

    /// 将数据库 Model 转换为核心领域 HostRecord (同时解密敏感字段)
    fn model_to_record(&self, m: host::Model) -> HostRecord {
        let password = m.password_enc.and_then(|p| self.vault.decrypt_string(&p).ok());
        let key_data = m.key_data_enc.and_then(|k| self.vault.decrypt_string(&k).ok());
        let key_passphrase = m.key_passphrase_enc.and_then(|kp| self.vault.decrypt_string(&kp).ok());
        let proxy_password = m.proxy_password_enc.and_then(|pp| self.vault.decrypt_string(&pp).ok());

        HostRecord {
            id: m.id,
            name: m.name,
            address: m.address,
            port: m.port as u16,
            parent_group_id: m.parent_group_id,
            credential_id: m.credential_id,
            status: HostStatus::from(m.status.as_str()),
            ping_ms: m.ping_ms,
            sort_order: m.sort_order,
            notes: m.notes,
            auth_type: m.auth_type,
            username: m.username,
            password,
            key_data,
            key_passphrase,
            proxy_type: m.proxy_type,
            proxy_host: m.proxy_host,
            proxy_port: m.proxy_port.map(|p| p as u16),
            proxy_username: m.proxy_username,
            proxy_password,
            jump_chain_summary: if m.jump_chain_json.is_empty() || m.jump_chain_json == "[]" {
                None
            } else {
                Some(m.jump_chain_json)
            },
            keepalive_interval: m.keepalive_interval as u32,
            connect_timeout: m.connect_timeout as u32,
            initial_dir: m.initial_dir,
            startup_cmd: m.startup_cmd,
            term_type: m.term_type,
        }
    }

    /// 将核心领域 HostRecord 转换为可插入/更新的 ActiveModel (同时加密敏感字段)
    fn record_to_active_model(&self, r: &HostRecord, existing: Option<host::Model>) -> StorageResult<host::ActiveModel> {
        let now = chrono::Utc::now().timestamp();
        let pass_enc = match &r.password {
            Some(p) if !p.is_empty() => Some(self.vault.encrypt_string(p).map_err(|e| StorageError::Backend(e.to_string()))?),
            _ => None,
        };
        let key_enc = match &r.key_data {
            Some(k) if !k.is_empty() => Some(self.vault.encrypt_string(k).map_err(|e| StorageError::Backend(e.to_string()))?),
            _ => None,
        };
        let kp_enc = match &r.key_passphrase {
            Some(kp) if !kp.is_empty() => Some(self.vault.encrypt_string(kp).map_err(|e| StorageError::Backend(e.to_string()))?),
            _ => None,
        };
        let pp_enc = match &r.proxy_password {
            Some(pp) if !pp.is_empty() => Some(self.vault.encrypt_string(pp).map_err(|e| StorageError::Backend(e.to_string()))?),
            _ => None,
        };

        let jump_json = r.jump_chain_summary.clone().unwrap_or_else(|| "[]".to_string());
        let keepalive = r.keepalive_interval as i32;
        let timeout = r.connect_timeout as i32;

        let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);
        let version = existing.as_ref().map(|m| m.version + 1).unwrap_or(1);

        Ok(host::ActiveModel {
            id: Set(r.id.clone()),
            workspace_id: Set("default".to_string()),
            name: Set(r.name.clone()),
            address: Set(r.address.clone()),
            port: Set(r.port as i32),
            parent_group_id: Set(r.parent_group_id.clone()),
            credential_id: Set(r.credential_id.clone()),
            status: Set(r.status.to_string()),
            ping_ms: Set(r.ping_ms),
            sort_order: Set(r.sort_order),
            notes: Set(r.notes.clone()),
            auth_type: Set(r.auth_type.clone()),
            username: Set(r.username.clone()),
            password_enc: Set(pass_enc),
            key_data_enc: Set(key_enc),
            key_passphrase_enc: Set(kp_enc),
            proxy_type: Set(r.proxy_type.clone()),
            proxy_host: Set(r.proxy_host.clone()),
            proxy_port: Set(r.proxy_port.map(|p| p as i32)),
            proxy_username: Set(r.proxy_username.clone()),
            proxy_password_enc: Set(pp_enc),
            jump_chain_json: Set(jump_json),
            keepalive_interval: Set(keepalive),
            connect_timeout: Set(timeout),
            initial_dir: Set(r.initial_dir.clone()),
            startup_cmd: Set(r.startup_cmd.clone()),
            term_type: Set(r.term_type.clone()),
            tags_json: Set("[]".to_string()),
            version: Set(version),
            created_at: Set(created_at),
            updated_at: Set(now),
            deleted_at: Set(None),
        })
    }
}

#[async_trait]
impl HostRepository for SeaOrmHostRepository {
    async fn list_all(&self) -> StorageResult<Vec<HostRecord>> {
        let models = Host::find()
            .filter(host::Column::DeletedAt.is_null())
            .order_by_asc(host::Column::SortOrder)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(|m| self.model_to_record(m)).collect())
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<HostRecord>> {
        let model = Host::find_by_id(id)
            .filter(host::Column::DeletedAt.is_null())
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(model.map(|m| self.model_to_record(m)))
    }

    async fn list_by_credential(&self, credential_id: &str) -> StorageResult<Vec<HostRecord>> {
        let models = Host::find()
            .filter(host::Column::DeletedAt.is_null())
            .filter(host::Column::CredentialId.eq(credential_id))
            .order_by_asc(host::Column::SortOrder)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(|m| self.model_to_record(m)).collect())
    }

    async fn save(&self, host: &HostRecord) -> StorageResult<()> {
        let existing = Host::find_by_id(&host.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if existing.is_some() {
            let active = self.record_to_active_model(host, existing)?;
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = self.record_to_active_model(host, None)?;
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn save_batch(&self, hosts: &[HostRecord]) -> StorageResult<()> {
        for h in hosts {
            self.save(h).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let res = Host::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(res.rows_affected > 0)
    }

    async fn update_list_order(&self, ordered_ids: &[String]) -> StorageResult<()> {
        for (i, id) in ordered_ids.iter().enumerate() {
            if let Some(existing) = Host::find_by_id(id)
                .one(&self.db)
                .await
                .map_err(|e| StorageError::Backend(e.to_string()))?
            {
                let mut active: host::ActiveModel = existing.into();
                active.sort_order = Set(i as i32);
                active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
            }
        }
        Ok(())
    }
}
