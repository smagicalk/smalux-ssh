//! 内存容灾备份任务与快照仓储实现

use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use smagical_core::domain::backup::{BackupSnapshotRecord, BackupTaskRecord};
use smagical_core::storage::{BackupSnapshotRepository, BackupTaskRepository, StorageError, StorageResult};

/// 线程安全的内存容灾备份任务仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockBackupTaskRepository {
    tasks: Arc<RwLock<Vec<BackupTaskRecord>>>,
}

impl MockBackupTaskRepository {
    /// 创建空的内存容灾备份任务仓储
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 使用指定任务列表创建内存仓储
    pub fn with_tasks(tasks: Vec<BackupTaskRecord>) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(tasks)),
        }
    }
}

#[async_trait]
impl BackupTaskRepository for MockBackupTaskRepository {
    async fn list_all(&self) -> StorageResult<Vec<BackupTaskRecord>> {
        let guard = self.tasks.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(guard.clone())
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<BackupTaskRecord>> {
        let guard = self.tasks.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(guard.iter().find(|t| t.id == id).cloned())
    }

    async fn save(&self, task: &BackupTaskRecord) -> StorageResult<()> {
        let mut guard = self.tasks.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = guard.iter().position(|t| t.id == task.id) {
            guard[pos] = task.clone();
        } else {
            guard.push(task.clone());
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut guard = self.tasks.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let len_before = guard.len();
        guard.retain(|t| t.id != id);
        Ok(guard.len() < len_before)
    }

    async fn set_enabled(&self, id: &str, enabled: bool) -> StorageResult<bool> {
        let mut guard = self.tasks.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(t) = guard.iter_mut().find(|t| t.id == id) {
            t.enabled = enabled;
            t.updated_at = chrono::Utc::now().timestamp() as u64;
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
        let mut guard = self.tasks.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(t) = guard.iter_mut().find(|t| t.id == id) {
            t.last_status = status.to_string();
            t.last_error = error.to_string();
            if !last_backup_time.is_empty() {
                t.last_backup_time = last_backup_time.to_string();
            }
            t.snapshot_count = snapshot_count;
            t.updated_at = chrono::Utc::now().timestamp() as u64;
        }
        Ok(())
    }
}

/// 线程安全的内存备份镜像快照仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockBackupSnapshotRepository {
    snapshots: Arc<RwLock<Vec<BackupSnapshotRecord>>>,
}

impl MockBackupSnapshotRepository {
    /// 创建空的内存备份镜像快照仓储
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 使用指定快照列表创建内存仓储
    pub fn with_snapshots(snapshots: Vec<BackupSnapshotRecord>) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(snapshots)),
        }
    }
}

#[async_trait]
impl BackupSnapshotRepository for MockBackupSnapshotRepository {
    async fn list_by_task(&self, task_id: &str) -> StorageResult<Vec<BackupSnapshotRecord>> {
        let guard = self.snapshots.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        let mut list: Vec<BackupSnapshotRecord> = guard.iter().filter(|s| s.task_id == task_id).cloned().collect();
        list.sort_by(|a, b| b.epoch_secs.cmp(&a.epoch_secs));
        Ok(list)
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<BackupSnapshotRecord>> {
        let guard = self.snapshots.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(guard.iter().find(|s| s.id == id).cloned())
    }

    async fn save(&self, snapshot: &BackupSnapshotRecord) -> StorageResult<()> {
        let mut guard = self.snapshots.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = guard.iter().position(|s| s.id == snapshot.id) {
            guard[pos] = snapshot.clone();
        } else {
            guard.push(snapshot.clone());
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut guard = self.snapshots.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let len_before = guard.len();
        guard.retain(|s| s.id != id);
        Ok(guard.len() < len_before)
    }

    async fn delete_by_task(&self, task_id: &str) -> StorageResult<usize> {
        let mut guard = self.snapshots.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let len_before = guard.len();
        guard.retain(|s| s.task_id != task_id);
        Ok(len_before - guard.len())
    }

    async fn prune_old_snapshots(&self, task_id: &str, keep_count: usize) -> StorageResult<Vec<BackupSnapshotRecord>> {
        if keep_count == 0 {
            return Ok(Vec::new());
        }
        let mut guard = self.snapshots.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let mut matching_indices: Vec<usize> = guard
            .iter()
            .enumerate()
            .filter(|(_, s)| s.task_id == task_id)
            .map(|(i, _)| i)
            .collect();

        matching_indices.sort_by(|&a, &b| guard[b].epoch_secs.cmp(&guard[a].epoch_secs));

        if matching_indices.len() <= keep_count {
            return Ok(Vec::new());
        }

        let to_remove_indices: std::collections::HashSet<usize> =
            matching_indices.into_iter().skip(keep_count).collect();

        let mut removed = Vec::new();
        let mut i = 0;
        guard.retain(|s| {
            let idx = i;
            i += 1;
            if to_remove_indices.contains(&idx) {
                removed.push(s.clone());
                false
            } else {
                true
            }
        });
        Ok(removed)
    }
}
