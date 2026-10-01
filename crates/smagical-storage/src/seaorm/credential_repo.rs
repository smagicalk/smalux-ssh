//! SeaORM 凭据保险库仓储实现 (SeaOrmCredentialRepository)。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use smagical_core::domain::credential::{CredentialRecord, CredentialType};
use smagical_core::storage::{CredentialRepository, StorageError, StorageResult};

use crate::crypto::VaultManager;
use crate::entities::credential;
use crate::entities::{Credential, Host};

/// 基于 SeaORM 的凭据保险库持久化仓储
#[derive(Clone)]
pub struct SeaOrmCredentialRepository {
    db: DatabaseConnection,
    vault: VaultManager,
}

impl SeaOrmCredentialRepository {
    /// 构造新凭据仓储实例
    pub fn new(db: DatabaseConnection, vault: VaultManager) -> Self {
        Self { db, vault }
    }

    /// 将数据库 Model 转换为核心领域 CredentialRecord (同时解密敏感密钥/密码)
    async fn model_to_record(&self, m: credential::Model) -> StorageResult<CredentialRecord> {
        let secret_data = self
            .vault
            .decrypt_string(&m.secret_data_enc)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let passphrase = match m.passphrase_enc {
            Some(ref p) if !p.is_empty() => Some(
                self.vault
                    .decrypt_string(p)
                    .map_err(|e| StorageError::Backend(e.to_string()))?,
            ),
            _ => None,
        };

        let bound_host_count = Host::find()
            .filter(crate::entities::host::Column::DeletedAt.is_null())
            .filter(crate::entities::host::Column::CredentialId.eq(&m.id))
            .count(&self.db)
            .await
            .unwrap_or(0) as usize;

        Ok(CredentialRecord {
            id: m.id,
            name: m.name,
            cred_type: CredentialType::from(m.cred_type.as_str()),
            algorithm: m.algorithm,
            username: m.username,
            secret_data,
            passphrase,
            public_key: m.public_key,
            fingerprint: m.fingerprint,
            bound_host_count,
            created_at: format!("{}", m.created_at),
            updated_at: format!("{}", m.updated_at),
            notes: String::new(),
        })
    }
}

#[async_trait]
impl CredentialRepository for SeaOrmCredentialRepository {
    async fn list_all(&self) -> StorageResult<Vec<CredentialRecord>> {
        let models = Credential::find()
            .filter(credential::Column::DeletedAt.is_null())
            .order_by_asc(credential::Column::Name)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut list = Vec::new();
        for m in models {
            list.push(self.model_to_record(m).await?);
        }
        Ok(list)
    }

    async fn list_by_type(&self, cred_type: CredentialType) -> StorageResult<Vec<CredentialRecord>> {
        let models = Credential::find()
            .filter(credential::Column::DeletedAt.is_null())
            .filter(credential::Column::CredType.eq(cred_type.as_str()))
            .order_by_asc(credential::Column::Name)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut list = Vec::new();
        for m in models {
            list.push(self.model_to_record(m).await?);
        }
        Ok(list)
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<CredentialRecord>> {
        let model = Credential::find_by_id(id)
            .filter(credential::Column::DeletedAt.is_null())
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match model {
            Some(m) => Ok(Some(self.model_to_record(m).await?)),
            None => Ok(None),
        }
    }

    async fn search(&self, query: &str) -> StorageResult<Vec<CredentialRecord>> {
        let q = format!("%{}%", query);
        let models = Credential::find()
            .filter(credential::Column::DeletedAt.is_null())
            .filter(
                credential::Column::Name.like(&q)
                    .or(credential::Column::Username.like(&q))
                    .or(credential::Column::Fingerprint.like(&q)),
            )
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut list = Vec::new();
        for m in models {
            list.push(self.model_to_record(m).await?);
        }
        Ok(list)
    }

    async fn get_bound_hosts(&self, id: &str) -> StorageResult<Vec<String>> {
        let hosts = Host::find()
            .filter(crate::entities::host::Column::DeletedAt.is_null())
            .filter(crate::entities::host::Column::CredentialId.eq(id))
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(hosts.into_iter().map(|h| h.id).collect())
    }

    async fn save(&self, r: &CredentialRecord) -> StorageResult<()> {
        let now = chrono::Utc::now().timestamp();
        let secret_enc = self
            .vault
            .encrypt_string(&r.secret_data)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let pass_enc = match &r.passphrase {
            Some(p) if !p.is_empty() => Some(
                self.vault
                    .encrypt_string(p)
                    .map_err(|e| StorageError::Backend(e.to_string()))?,
            ),
            _ => None,
        };

        let existing = Credential::find_by_id(&r.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: credential::ActiveModel = m.into();
            active.name = Set(r.name.clone());
            active.cred_type = Set(r.cred_type.as_str().to_string());
            active.algorithm = Set(r.algorithm.clone());
            active.username = Set(r.username.clone());
            active.secret_data_enc = Set(secret_enc);
            active.passphrase_enc = Set(pass_enc);
            active.public_key = Set(r.public_key.clone());
            active.fingerprint = Set(r.fingerprint.clone());
            active.updated_at = Set(now);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = credential::ActiveModel {
                id: Set(r.id.clone()),
                workspace_id: Set("default".to_string()),
                name: Set(r.name.clone()),
                cred_type: Set(r.cred_type.as_str().to_string()),
                algorithm: Set(r.algorithm.clone()),
                username: Set(r.username.clone()),
                secret_data_enc: Set(secret_enc),
                passphrase_enc: Set(pass_enc),
                public_key: Set(r.public_key.clone()),
                fingerprint: Set(r.fingerprint.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn save_batch(&self, records: &[CredentialRecord]) -> StorageResult<()> {
        for r in records {
            self.save(r).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let res = Credential::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(res.rows_affected > 0)
    }
}
