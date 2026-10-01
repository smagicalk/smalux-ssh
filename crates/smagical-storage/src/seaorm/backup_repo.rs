//! SeaORM 容灾备份任务与快照仓储实现 (SeaOrmBackupTaskRepository, SeaOrmBackupSnapshotRepository)。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use smagical_core::domain::backup::{BackupSnapshotRecord, BackupStrategy, BackupTaskRecord, BackupType};
use smagical_core::storage::{BackupSnapshotRepository, BackupTaskRepository, StorageError, StorageResult};

use crate::crypto::VaultManager;
use crate::entities::backup_snapshot;
use crate::entities::backup_task;
use crate::entities::{BackupSnapshot, BackupTask};

/// 基于 SeaORM 的容灾备份任务仓储
#[derive(Clone)]
pub struct SeaOrmBackupTaskRepository {
    db: DatabaseConnection,
    vault: VaultManager,
}

impl SeaOrmBackupTaskRepository {
    /// 创建备份任务仓储
    pub fn new(db: DatabaseConnection, vault: VaultManager) -> Self {
        Self { db, vault }
    }

    fn model_to_record(&self, m: backup_task::Model) -> BackupTaskRecord {
        let auth_secret = self.vault.decrypt_string(&m.auth_secret_enc).unwrap_or_default();
        BackupTaskRecord {
            id: m.id,
            name: m.name,
            backup_type: BackupType::from_str(&m.backup_type),
            endpoint: m.endpoint,
            auth_user: m.auth_user,
            auth_secret,
            strategy: BackupStrategy::from_str(&m.strategy),
            retention: m.retention,
            enabled: m.enabled,
            last_backup_time: m.last_backup_time,
            snapshot_count: m.snapshot_count as u32,
            last_status: m.last_status,
            last_error: m.last_error,
            created_at: m.created_at as u64,
            updated_at: m.updated_at as u64,
        }
    }
}

#[async_trait]
impl BackupTaskRepository for SeaOrmBackupTaskRepository {
    async fn list_all(&self) -> StorageResult<Vec<BackupTaskRecord>> {
        let models = BackupTask::find()
            .order_by_desc(backup_task::Column::UpdatedAt)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(|m| self.model_to_record(m)).collect())
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<BackupTaskRecord>> {
        let model = BackupTask::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(model.map(|m| self.model_to_record(m)))
    }

    async fn save(&self, task: &BackupTaskRecord) -> StorageResult<()> {
        let now = chrono::Utc::now().timestamp();
        let auth_secret_enc = self
            .vault
            .encrypt_string(&task.auth_secret)
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let existing = BackupTask::find_by_id(&task.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: backup_task::ActiveModel = m.into();
            active.name = Set(task.name.clone());
            active.backup_type = Set(task.backup_type.as_str().to_string());
            active.endpoint = Set(task.endpoint.clone());
            active.auth_user = Set(task.auth_user.clone());
            if !task.auth_secret.is_empty() {
                active.auth_secret_enc = Set(auth_secret_enc);
            }
            active.strategy = Set(task.strategy.as_str().to_string());
            active.retention = Set(task.retention.clone());
            active.enabled = Set(task.enabled);
            active.last_backup_time = Set(task.last_backup_time.clone());
            active.snapshot_count = Set(task.snapshot_count as i32);
            active.last_status = Set(task.last_status.clone());
            active.last_error = Set(task.last_error.clone());
            active.updated_at = Set(now);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = backup_task::ActiveModel {
                id: Set(task.id.clone()),
                name: Set(task.name.clone()),
                backup_type: Set(task.backup_type.as_str().to_string()),
                endpoint: Set(task.endpoint.clone()),
                auth_user: Set(task.auth_user.clone()),
                auth_secret_enc: Set(auth_secret_enc),
                strategy: Set(task.strategy.as_str().to_string()),
                retention: Set(task.retention.clone()),
                enabled: Set(task.enabled),
                last_backup_time: Set(task.last_backup_time.clone()),
                snapshot_count: Set(task.snapshot_count as i32),
                last_status: Set(task.last_status.clone()),
                last_error: Set(task.last_error.clone()),
                created_at: Set(if task.created_at > 0 { task.created_at as i64 } else { now }),
                updated_at: Set(now),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let res = BackupTask::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(res.rows_affected > 0)
    }

    async fn set_enabled(&self, id: &str, enabled: bool) -> StorageResult<bool> {
        let existing = BackupTask::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: backup_task::ActiveModel = m.into();
            active.enabled = Set(enabled);
            active.updated_at = Set(chrono::Utc::now().timestamp());
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn update_status(
        &self,
        id: &str,
        status: &str,
        error: &str,
        last_backup_time: &str,
        snapshot_count: u32,
    ) -> StorageResult<()> {
        let existing = BackupTask::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: backup_task::ActiveModel = m.into();
            active.last_status = Set(status.to_string());
            active.last_error = Set(error.to_string());
            if !last_backup_time.is_empty() {
                active.last_backup_time = Set(last_backup_time.to_string());
            }
            active.snapshot_count = Set(snapshot_count as i32);
            active.updated_at = Set(chrono::Utc::now().timestamp());
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }
        Ok(())
    }
}

/// 基于 SeaORM 的容灾备份快照仓储
#[derive(Clone)]
pub struct SeaOrmBackupSnapshotRepository {
    db: DatabaseConnection,
}

impl SeaOrmBackupSnapshotRepository {
    /// 创建备份快照仓储
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn model_to_record(m: backup_snapshot::Model) -> BackupSnapshotRecord {
        BackupSnapshotRecord {
            id: m.id,
            task_id: m.task_id,
            timestamp: m.timestamp,
            epoch_secs: m.epoch_secs as u64,
            size_str: m.size_str,
            size_bytes: m.size_bytes as u64,
            remark: m.remark,
            hash: m.hash,
            remote_id: m.remote_id,
        }
    }
}

#[async_trait]
impl BackupSnapshotRepository for SeaOrmBackupSnapshotRepository {
    async fn list_by_task(&self, task_id: &str) -> StorageResult<Vec<BackupSnapshotRecord>> {
        let models = BackupSnapshot::find()
            .filter(backup_snapshot::Column::TaskId.eq(task_id))
            .order_by_desc(backup_snapshot::Column::EpochSecs)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(Self::model_to_record).collect())
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<BackupSnapshotRecord>> {
        let model = BackupSnapshot::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(model.map(Self::model_to_record))
    }

    async fn save(&self, snapshot: &BackupSnapshotRecord) -> StorageResult<()> {
        let existing = BackupSnapshot::find_by_id(&snapshot.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: backup_snapshot::ActiveModel = m.into();
            active.task_id = Set(snapshot.task_id.clone());
            active.timestamp = Set(snapshot.timestamp.clone());
            active.epoch_secs = Set(snapshot.epoch_secs as i64);
            active.size_str = Set(snapshot.size_str.clone());
            active.size_bytes = Set(snapshot.size_bytes as i64);
            active.remark = Set(snapshot.remark.clone());
            active.hash = Set(snapshot.hash.clone());
            active.remote_id = Set(snapshot.remote_id.clone());
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = backup_snapshot::ActiveModel {
                id: Set(snapshot.id.clone()),
                task_id: Set(snapshot.task_id.clone()),
                timestamp: Set(snapshot.timestamp.clone()),
                epoch_secs: Set(snapshot.epoch_secs as i64),
                size_str: Set(snapshot.size_str.clone()),
                size_bytes: Set(snapshot.size_bytes as i64),
                remark: Set(snapshot.remark.clone()),
                hash: Set(snapshot.hash.clone()),
                remote_id: Set(snapshot.remote_id.clone()),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let res = BackupSnapshot::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(res.rows_affected > 0)
    }

    async fn delete_by_task(&self, task_id: &str) -> StorageResult<usize> {
        let res = BackupSnapshot::delete_many()
            .filter(backup_snapshot::Column::TaskId.eq(task_id))
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(res.rows_affected as usize)
    }

    async fn prune_old_snapshots(&self, task_id: &str, keep_count: usize) -> StorageResult<Vec<BackupSnapshotRecord>> {
        if keep_count == 0 {
            return Ok(Vec::new());
        }
        let all = BackupSnapshot::find()
            .filter(backup_snapshot::Column::TaskId.eq(task_id))
            .order_by_desc(backup_snapshot::Column::EpochSecs)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if all.len() <= keep_count {
            return Ok(Vec::new());
        }

        let to_remove = &all[keep_count..];
        let mut pruned = Vec::new();
        for m in to_remove {
            let rec = Self::model_to_record(m.clone());
            let _ = BackupSnapshot::delete_by_id(&m.id).exec(&self.db).await;
            pruned.push(rec);
        }
        Ok(pruned)
    }
}
