//! SeaORM 分组层级仓储实现 (SeaOrmGroupRepository)。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use smagical_core::domain::group::GroupRecord;
use smagical_core::storage::{GroupRepository, StorageError, StorageResult};

use crate::entities::group;
use crate::entities::Group;

/// 基于 SeaORM 的分组层级持久化仓储
#[derive(Clone)]
pub struct SeaOrmGroupRepository {
    db: DatabaseConnection,
}

impl SeaOrmGroupRepository {
    /// 构造新分组仓储实例
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn model_to_record(m: group::Model) -> GroupRecord {
        GroupRecord {
            id: m.id,
            name: m.name,
            parent_id: m.parent_id,
            level: m.level,
            is_expanded: m.is_expanded,
            sort_order: m.sort_order,
        }
    }
}

#[async_trait]
impl GroupRepository for SeaOrmGroupRepository {
    async fn list_all(&self) -> StorageResult<Vec<GroupRecord>> {
        let models = Group::find()
            .filter(group::Column::DeletedAt.is_null())
            .order_by_asc(group::Column::Level)
            .order_by_asc(group::Column::SortOrder)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(Self::model_to_record).collect())
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<GroupRecord>> {
        let model = Group::find_by_id(id)
            .filter(group::Column::DeletedAt.is_null())
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(model.map(Self::model_to_record))
    }

    async fn save(&self, g: &GroupRecord) -> StorageResult<()> {
        let now = chrono::Utc::now().timestamp();
        let existing = Group::find_by_id(&g.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: group::ActiveModel = m.into();
            active.name = Set(g.name.clone());
            active.parent_id = Set(g.parent_id.clone());
            active.level = Set(g.level);
            active.is_expanded = Set(g.is_expanded);
            active.sort_order = Set(g.sort_order);
            active.updated_at = Set(now);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = group::ActiveModel {
                id: Set(g.id.clone()),
                workspace_id: Set("default".to_string()),
                name: Set(g.name.clone()),
                parent_id: Set(g.parent_id.clone()),
                level: Set(g.level),
                is_expanded: Set(g.is_expanded),
                sort_order: Set(g.sort_order),
                color_badge: Set(String::new()),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let res = Group::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(res.rows_affected > 0)
    }

    async fn set_expanded(&self, id: &str, expanded: bool) -> StorageResult<()> {
        if let Some(m) = Group::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let mut active: group::ActiveModel = m.into();
            active.is_expanded = Set(expanded);
            active.updated_at = Set(chrono::Utc::now().timestamp());
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }
        Ok(())
    }

    async fn move_group(&self, id: &str, new_parent_id: Option<&str>) -> StorageResult<()> {
        let new_level = if let Some(pid) = new_parent_id {
            if let Some(parent) = Group::find_by_id(pid)
                .one(&self.db)
                .await
                .map_err(|e| StorageError::Backend(e.to_string()))?
            {
                parent.level + 1
            } else {
                0
            }
        } else {
            0
        };

        if let Some(m) = Group::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let mut active: group::ActiveModel = m.into();
            active.parent_id = Set(new_parent_id.map(|s| s.to_string()));
            active.level = Set(new_level);
            active.updated_at = Set(chrono::Utc::now().timestamp());
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }
        Ok(())
    }
}
