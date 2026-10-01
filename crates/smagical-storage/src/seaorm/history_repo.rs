//! SeaORM 历史会话仓储实现 (SeaOrmHistoryRepository)。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use smagical_core::domain::history::HistoryRecord;
use smagical_core::storage::{HistoryRepository, StorageError, StorageResult};

use crate::entities::history;
use crate::entities::snapshot;
use crate::entities::{History, HistorySnapshot};

/// 基于 SeaORM 的历史会话持久化仓储
#[derive(Clone)]
pub struct SeaOrmHistoryRepository {
    db: DatabaseConnection,
}

impl SeaOrmHistoryRepository {
    /// 构造新历史仓储实例
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn model_to_record(m: history::Model) -> HistoryRecord {
        HistoryRecord {
            id: m.id,
            host_id: m.host_id,
            title: m.title,
            address: m.address,
            port: m.port as u16,
            username: m.username,
            session_type: m.session_type,
            connected_at: m.connected_at as u64,
            disconnected_at: m.disconnected_at.map(|d| d as u64),
            duration_secs: m.duration_secs as u64,
            exit_status: m.exit_status,
            error_msg: m.error_msg,
            is_pinned: m.is_pinned,
            connect_count: m.connect_count as u32,
            has_snapshot: m.has_snapshot,
            snapshot_lines: m.snapshot_lines as u32,
        }
    }
}

#[async_trait]
impl HistoryRepository for SeaOrmHistoryRepository {
    async fn list_all(&self) -> StorageResult<Vec<HistoryRecord>> {
        let models = History::find()
            .order_by_desc(history::Column::ConnectedAt)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(Self::model_to_record).collect())
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<HistoryRecord>> {
        let model = History::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(model.map(Self::model_to_record))
    }

    async fn save(&self, r: &HistoryRecord) -> StorageResult<()> {
        let existing = History::find_by_id(&r.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: history::ActiveModel = m.into();
            active.title = Set(r.title.clone());
            active.address = Set(r.address.clone());
            active.port = Set(r.port as i32);
            active.username = Set(r.username.clone());
            active.session_type = Set(r.session_type.clone());
            active.disconnected_at = Set(r.disconnected_at.map(|d| d as i64));
            active.duration_secs = Set(r.duration_secs as i64);
            active.exit_status = Set(r.exit_status.clone());
            active.error_msg = Set(r.error_msg.clone());
            active.is_pinned = Set(r.is_pinned);
            active.connect_count = Set(r.connect_count as i32);
            active.has_snapshot = Set(r.has_snapshot);
            active.snapshot_lines = Set(r.snapshot_lines as i32);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = history::ActiveModel {
                id: Set(r.id.clone()),
                workspace_id: Set("default".to_string()),
                host_id: Set(r.host_id.clone()),
                title: Set(r.title.clone()),
                address: Set(r.address.clone()),
                port: Set(r.port as i32),
                username: Set(r.username.clone()),
                session_type: Set(r.session_type.clone()),
                connected_at: Set(r.connected_at as i64),
                disconnected_at: Set(r.disconnected_at.map(|d| d as i64)),
                duration_secs: Set(r.duration_secs as i64),
                exit_status: Set(r.exit_status.clone()),
                error_msg: Set(r.error_msg.clone()),
                is_pinned: Set(r.is_pinned),
                connect_count: Set(r.connect_count as i32),
                has_snapshot: Set(r.has_snapshot),
                snapshot_lines: Set(r.snapshot_lines as i32),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn save_batch(&self, records: &[HistoryRecord]) -> StorageResult<()> {
        for r in records {
            self.save(r).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let _ = HistorySnapshot::delete_by_id(id).exec(&self.db).await;
        let res = History::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(res.rows_affected > 0)
    }

    async fn clear_all(&self, keep_pinned: bool) -> StorageResult<()> {
        if keep_pinned {
            let unpinned = History::find()
                .filter(history::Column::IsPinned.eq(false))
                .all(&self.db)
                .await
                .map_err(|e| StorageError::Backend(e.to_string()))?;

            for item in unpinned {
                let _ = HistorySnapshot::delete_by_id(&item.id).exec(&self.db).await;
                let _ = History::delete_by_id(&item.id).exec(&self.db).await;
            }
        } else {
            let _ = HistorySnapshot::delete_many().exec(&self.db).await;
            let _ = History::delete_many().exec(&self.db).await;
        }

        Ok(())
    }

    async fn toggle_pin(&self, id: &str) -> StorageResult<bool> {
        if let Some(m) = History::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let new_pin = !m.is_pinned;
            let mut active: history::ActiveModel = m.into();
            active.is_pinned = Set(new_pin);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok(new_pin)
        } else {
            Err(StorageError::NotFound(id.to_string()))
        }
    }

    async fn save_snapshot(&self, history_id: &str, content: &str, max_lines: usize) -> StorageResult<()> {
        let now = chrono::Utc::now().timestamp();
        let existing = HistorySnapshot::find_by_id(history_id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: snapshot::ActiveModel = m.into();
            active.content = Set(content.to_string());
            active.max_lines = Set(max_lines as i32);
            active.saved_at = Set(now);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = snapshot::ActiveModel {
                history_id: Set(history_id.to_string()),
                content: Set(content.to_string()),
                max_lines: Set(max_lines as i32),
                saved_at: Set(now),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        // 同步更新主表的 has_snapshot 状态
        if let Some(h) = History::find_by_id(history_id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let line_count = content.lines().count() as i32;
            let mut active: history::ActiveModel = h.into();
            active.has_snapshot = Set(true);
            active.snapshot_lines = Set(line_count);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn get_snapshot(&self, history_id: &str) -> StorageResult<Option<String>> {
        let model = HistorySnapshot::find_by_id(history_id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(model.map(|m| m.content))
    }

    async fn delete_snapshot(&self, history_id: &str) -> StorageResult<bool> {
        let res = HistorySnapshot::delete_by_id(history_id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(h) = History::find_by_id(history_id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let mut active: history::ActiveModel = h.into();
            active.has_snapshot = Set(false);
            active.snapshot_lines = Set(0);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(res.rows_affected > 0)
    }
}
