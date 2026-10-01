//! SeaORM 网络隧道与代理仓储实现 (SeaOrmTunnelRepository)。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, QueryFilter, QueryOrder, Set,
};
use smagical_core::domain::tunnel::{TunnelRecord, TunnelType};
use smagical_core::storage::{StorageError, StorageResult, TunnelRepository};

use crate::entities::tunnel;
use crate::entities::Tunnel;

/// 基于 SeaORM 的网络隧道与代理持久化仓储
#[derive(Clone)]
pub struct SeaOrmTunnelRepository {
    db: DatabaseConnection,
}

impl SeaOrmTunnelRepository {
    /// 构造新网络隧道仓储实例
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn model_to_record(m: tunnel::Model) -> StorageResult<TunnelRecord> {
        if !m.record_json.is_empty() {
            if let Ok(mut r) = serde_json::from_str::<TunnelRecord>(&m.record_json) {
                // 以最新物理列覆盖状态
                r.is_running = m.is_running;
                return Ok(r);
            }
        }

        Err(StorageError::Serialization("解析隧道 JSON 数据失败".into()))
    }
}

#[async_trait]
impl TunnelRepository for SeaOrmTunnelRepository {
    async fn list_all(&self) -> StorageResult<Vec<TunnelRecord>> {
        let models = Tunnel::find()
            .order_by_asc(tunnel::Column::Name)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut list = Vec::new();
        for m in models {
            if let Ok(r) = Self::model_to_record(m) {
                list.push(r);
            }
        }
        Ok(list)
    }

    async fn list_by_type(&self, tunnel_type: TunnelType) -> StorageResult<Vec<TunnelRecord>> {
        let models = Tunnel::find()
            .filter(tunnel::Column::TunnelType.eq(tunnel_type.as_str()))
            .order_by_asc(tunnel::Column::Name)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut list = Vec::new();
        for m in models {
            if let Ok(r) = Self::model_to_record(m) {
                list.push(r);
            }
        }
        Ok(list)
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<TunnelRecord>> {
        let model = Tunnel::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match model {
            Some(m) => Ok(Self::model_to_record(m).ok()),
            None => Ok(None),
        }
    }

    async fn search(&self, query: &str) -> StorageResult<Vec<TunnelRecord>> {
        let q = format!("%{}%", query);
        let models = Tunnel::find()
            .filter(
                tunnel::Column::Name.like(&q)
                    .or(tunnel::Column::RemoteHost.like(&q))
                    .or(tunnel::Column::Notes.like(&q)),
            )
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let mut list = Vec::new();
        for m in models {
            if let Ok(r) = Self::model_to_record(m) {
                list.push(r);
            }
        }
        Ok(list)
    }

    async fn save(&self, r: &TunnelRecord) -> StorageResult<()> {
        let now = chrono::Utc::now().timestamp();
        let record_json = serde_json::to_string(r).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let jump_json = serde_json::to_string(&r.jump_chain).unwrap_or_else(|_| "[]".to_string());

        let existing = Tunnel::find_by_id(&r.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: tunnel::ActiveModel = m.into();
            active.name = Set(r.name.clone());
            active.tunnel_type = Set(r.tunnel_type.as_str().to_string());
            active.local_bind = Set(r.local_bind.clone());
            active.local_port = Set(r.local_port as i32);
            active.remote_host = Set(r.remote_host.clone());
            active.remote_port = Set(r.remote_port as i32);
            active.associated_host_id = Set(r.ssh_host_id.clone());
            active.run_mode = Set(r.run_mode.as_str().to_string());
            active.is_running = Set(r.is_running);
            active.jump_hops_json = Set(jump_json);
            active.record_json = Set(record_json);
            active.notes = Set(r.notes.clone());
            active.updated_at = Set(now);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = tunnel::ActiveModel {
                id: Set(r.id.clone()),
                workspace_id: Set("default".to_string()),
                name: Set(r.name.clone()),
                tunnel_type: Set(r.tunnel_type.as_str().to_string()),
                local_bind: Set(r.local_bind.clone()),
                local_port: Set(r.local_port as i32),
                remote_host: Set(r.remote_host.clone()),
                remote_port: Set(r.remote_port as i32),
                associated_host_id: Set(r.ssh_host_id.clone()),
                run_mode: Set(r.run_mode.as_str().to_string()),
                is_running: Set(r.is_running),
                jump_hops_json: Set(jump_json),
                record_json: Set(record_json),
                notes: Set(r.notes.clone()),
                updated_at: Set(now),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn save_batch(&self, records: &[TunnelRecord]) -> StorageResult<()> {
        for r in records {
            self.save(r).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let res = Tunnel::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(res.rows_affected > 0)
    }

    async fn set_running(&self, id: &str, is_running: bool) -> StorageResult<bool> {
        if let Some(m) = Tunnel::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let mut active: tunnel::ActiveModel = m.into();
            active.is_running = Set(is_running);
            active.updated_at = Set(chrono::Utc::now().timestamp());
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn update_metrics(&self, id: &str, active_conn: usize, bytes_in: u64, bytes_out: u64) -> StorageResult<()> {
        if let Some(m) = Tunnel::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            if let Ok(mut r) = Self::model_to_record(m.clone()) {
                r.active_connections = active_conn;
                r.total_bytes_in = bytes_in;
                r.total_bytes_out = bytes_out;
                let record_json = serde_json::to_string(&r).unwrap_or_default();
                let mut active: tunnel::ActiveModel = m.into();
                active.record_json = Set(record_json);
                active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
            }
        }
        Ok(())
    }
}
