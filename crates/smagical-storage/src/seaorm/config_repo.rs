//! SeaORM 全局偏好与系统配置仓储实现 (SeaOrmConfigRepository)。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, Set,
};
use smagical_core::domain::config::AppConfigRecord;
use smagical_core::storage::{ConfigRepository, StorageError, StorageResult};

use crate::entities::config;
use crate::entities::AppConfig;

/// 基于 SeaORM 的全局配置持久化仓储
#[derive(Clone)]
pub struct SeaOrmConfigRepository {
    db: DatabaseConnection,
}

impl SeaOrmConfigRepository {
    /// 构造新全局配置仓储实例
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ConfigRepository for SeaOrmConfigRepository {
    async fn get(&self) -> StorageResult<AppConfigRecord> {
        let model = AppConfig::find_by_id("singleton")
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        match model {
            Some(m) => serde_json::from_str(&m.config_json)
                .map_err(|e| StorageError::Serialization(e.to_string())),
            None => Ok(AppConfigRecord::default()),
        }
    }

    async fn save(&self, config: &AppConfigRecord) -> StorageResult<()> {
        let now = chrono::Utc::now().timestamp();
        let config_json = serde_json::to_string(config)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let existing = AppConfig::find_by_id("singleton")
            .one(&self.db)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if let Some(m) = existing {
            let mut active: config::ActiveModel = m.into();
            active.config_json = Set(config_json);
            active.updated_at = Set(now);
            active.update(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        } else {
            let active = config::ActiveModel {
                id: Set("singleton".to_string()),
                config_json: Set(config_json),
                version: Set(1),
                updated_at: Set(now),
            };
            active.insert(&self.db).await.map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn reset_to_default(&self) -> StorageResult<AppConfigRecord> {
        let def = AppConfigRecord::default();
        self.save(&def).await?;
        Ok(def)
    }

    async fn update(
        &self,
        mutate: Box<dyn for<'a> FnOnce(&'a mut AppConfigRecord) + Send>,
    ) -> StorageResult<AppConfigRecord> {
        let mut curr = self.get().await?;
        mutate(&mut curr);
        self.save(&curr).await?;
        Ok(curr)
    }
}
