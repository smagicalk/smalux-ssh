//! 内存配置仓储实现

use std::sync::{Arc, RwLock};
use crate::domain::config::AppConfigRecord;
use crate::storage::{ConfigRepository, StorageError, StorageResult};

/// 线程安全的内存偏好配置仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockConfigRepository {
    config: Arc<RwLock<AppConfigRecord>>,
}

impl MockConfigRepository {
    /// 创建带有默认应用偏好配置的内存配置仓储
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(AppConfigRecord::default())),
        }
    }

    /// 使用自定义配置记录创建内存配置仓储
    pub fn with_config(config: AppConfigRecord) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }
}

impl ConfigRepository for MockConfigRepository {
    fn get(&self) -> StorageResult<AppConfigRecord> {
        let guard = self.config.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(guard.clone())
    }

    fn save(&self, config: &AppConfigRecord) -> StorageResult<()> {
        let mut guard = self.config.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        *guard = config.clone();
        Ok(())
    }

    fn reset_to_default(&self) -> StorageResult<AppConfigRecord> {
        let mut guard = self.config.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let default_config = AppConfigRecord::default();
        *guard = default_config.clone();
        Ok(default_config)
    }

    fn update(&self, mutate: Box<dyn FnOnce(&mut AppConfigRecord) + Send>) -> StorageResult<AppConfigRecord> {
        let mut guard = self.config.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        mutate(&mut *guard);
        Ok(guard.clone())
    }
}
