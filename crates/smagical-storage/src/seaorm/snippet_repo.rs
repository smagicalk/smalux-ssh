//! SeaORM 代码片段与分组仓储实现 (SeaOrmSnippetRepository)。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, QueryFilter, QueryOrder, Set,
};
use smagical_core::domain::snippet::{SnippetGroupRecord, SnippetRecord};
use smagical_core::storage::{SnippetRepository, StorageError, StorageResult};

use crate::entities::snippet;
use crate::entities::snippet_group;
use crate::entities::{Snippet, SnippetGroup};

/// 基于 SeaORM 的代码片段与分组持久化仓储
#[derive(Clone)]
pub struct SeaOrmSnippetRepository {
    db: DatabaseConnection,
}

impl SeaOrmSnippetRepository {
    /// 构造新代码片段仓储实例
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn model_to_record(m: snippet::Model) -> SnippetRecord {
        let tags: Vec<String> = serde_json::from_str(&m.tags_json).unwrap_or_default();
        SnippetRecord {
            id: m.id,
            parent_group_id: m.parent_group_id,
            title: m.title,
            content: m.content,
            language: m.language,
            tags,
            auto_execute: m.auto_execute,
            description: m.description,
            is_favorite: m.is_favorite,
            sort_order: m.sort_order,
            updated_at: format!("{}", m.updated_at),
        }
    }

    fn group_model_to_record(m: snippet_group::Model) -> SnippetGroupRecord {
        SnippetGroupRecord {
            id: m.id,
            name: m.name,
            parent_id: m.parent_id,
            level: m.level as u32,
            is_expanded: m.is_expanded,
            sort_order: m.sort_order,
        }
    }
}

#[async_trait]
impl SnippetRepository for SeaOrmSnippetRepository {
    async fn list_all(&self) -> StorageResult<Vec<SnippetRecord>> {
        let models = Snippet::find()
            .order_by_asc(snippet::Column::SortOrder)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(Self::model_to_record).collect())
    }

    async fn list_by_group(&self, group_id: Option<&str>) -> StorageResult<Vec<SnippetRecord>> {
        let query = Snippet::find();
        let models = match group_id {
            Some(gid) => query.filter(snippet::Column::ParentGroupId.eq(gid)),
            None => query.filter(snippet::Column::ParentGroupId.is_null()),
        }
        .order_by_asc(snippet::Column::SortOrder)
        .all(&self.db)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(Self::model_to_record).collect())
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<SnippetRecord>> {
        let model = Snippet::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(model.map(Self::model_to_record))
    }

    async fn search(&self, query: &str) -> StorageResult<Vec<SnippetRecord>> {
        let q = format!("%{}%", query);
        let models = Snippet::find()
            .filter(
                snippet::Column::Title.like(&q)
                    .or(snippet::Column::Content.like(&q))
                    .or(snippet::Column::Description.like(&q))
                    .or(snippet::Column::TagsJson.like(&q)),
            )
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(Self::model_to_record).collect())
    }

    async fn save(&self, r: &SnippetRecord) -> StorageResult<()> {
        let now = chrono::Utc::now().timestamp();
        let tags_json = serde_json::to_string(&r.tags).unwrap_or_else(|_| "[]".to_string());

        let existing = Snippet::find_by_id(&r.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: snippet::ActiveModel = m.into();
            active.parent_group_id = Set(r.parent_group_id.clone());
            active.title = Set(r.title.clone());
            active.content = Set(r.content.clone());
            active.language = Set(r.language.clone());
            active.tags_json = Set(tags_json);
            active.auto_execute = Set(r.auto_execute);
            active.description = Set(r.description.clone());
            active.is_favorite = Set(r.is_favorite);
            active.sort_order = Set(r.sort_order);
            active.updated_at = Set(now);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = snippet::ActiveModel {
                id: Set(r.id.clone()),
                workspace_id: Set("default".to_string()),
                parent_group_id: Set(r.parent_group_id.clone()),
                title: Set(r.title.clone()),
                content: Set(r.content.clone()),
                language: Set(r.language.clone()),
                tags_json: Set(tags_json),
                auto_execute: Set(r.auto_execute),
                description: Set(r.description.clone()),
                is_favorite: Set(r.is_favorite),
                sort_order: Set(r.sort_order),
                updated_at: Set(now),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn save_batch(&self, records: &[SnippetRecord]) -> StorageResult<()> {
        for r in records {
            self.save(r).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let res = Snippet::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(res.rows_affected > 0)
    }

    async fn toggle_favorite(&self, id: &str) -> StorageResult<bool> {
        if let Some(m) = Snippet::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let new_fav = !m.is_favorite;
            let mut active: snippet::ActiveModel = m.into();
            active.is_favorite = Set(new_fav);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok(new_fav)
        } else {
            Err(StorageError::NotFound(id.to_string()))
        }
    }

    async fn list_groups(&self) -> StorageResult<Vec<SnippetGroupRecord>> {
        let models = SnippetGroup::find()
            .order_by_asc(snippet_group::Column::Level)
            .order_by_asc(snippet_group::Column::SortOrder)
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(models.into_iter().map(Self::group_model_to_record).collect())
    }

    async fn get_group_by_id(&self, id: &str) -> StorageResult<Option<SnippetGroupRecord>> {
        let model = SnippetGroup::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(model.map(Self::group_model_to_record))
    }

    async fn save_group(&self, g: &SnippetGroupRecord) -> StorageResult<()> {
        let now = chrono::Utc::now().timestamp();
        let existing = SnippetGroup::find_by_id(&g.id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: snippet_group::ActiveModel = m.into();
            active.name = Set(g.name.clone());
            active.parent_id = Set(g.parent_id.clone());
            active.level = Set(g.level as i32);
            active.is_expanded = Set(g.is_expanded);
            active.sort_order = Set(g.sort_order);
            active.updated_at = Set(now);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = snippet_group::ActiveModel {
                id: Set(g.id.clone()),
                workspace_id: Set("default".to_string()),
                name: Set(g.name.clone()),
                parent_id: Set(g.parent_id.clone()),
                level: Set(g.level as i32),
                is_expanded: Set(g.is_expanded),
                sort_order: Set(g.sort_order),
                created_at: Set(now),
                updated_at: Set(now),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn delete_group(&self, id: &str) -> StorageResult<bool> {
        // 将子片段的 parent_group_id 回退置空
        let children = Snippet::find()
            .filter(snippet::Column::ParentGroupId.eq(id))
            .all(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        for c in children {
            let mut active: snippet::ActiveModel = c.into();
            active.parent_group_id = Set(None);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        let res = SnippetGroup::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(res.rows_affected > 0)
    }

    async fn set_group_expanded(&self, id: &str, expanded: bool) -> StorageResult<()> {
        if let Some(m) = SnippetGroup::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let mut active: snippet_group::ActiveModel = m.into();
            active.is_expanded = Set(expanded);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }
        Ok(())
    }

    async fn move_group(&self, id: &str, new_parent_id: Option<&str>) -> StorageResult<()> {
        let new_level = if let Some(pid) = new_parent_id {
            if let Some(parent) = SnippetGroup::find_by_id(pid)
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

        if let Some(m) = SnippetGroup::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?
        {
            let mut active: snippet_group::ActiveModel = m.into();
            active.parent_id = Set(new_parent_id.map(|s| s.to_string()));
            active.level = Set(new_level);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }
        Ok(())
    }
}
