//! Slint 应用根状态。

use std::fmt;

use crate::backend::{
    BackendCommandQueue, SharedBackendExecutor, noop_shared_backend_executor,
    shared_backend_executor,
};
use crate::config::AppConfig;
use crate::session::SessionManager;
use crate::storage::{RedbStorage, StorageManager, StoragePersistenceError};
use crate::terminal::TerminalManager;

use super::super::UiState;

/// Slint 应用的根状态。
///
/// 根状态只组合各个单一职责管理器，不直接实现 SSH、SFTP 或终端细节。
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionManager,
    pub storage: StorageManager,
    pub storage_backend: Option<RedbStorage>,
    pub terminal: TerminalManager,
    pub ui: UiState,
    pub backend_commands: BackendCommandQueue,
    pub backend_executor: SharedBackendExecutor,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .field("sessions", &self.sessions)
            .field("storage", &self.storage)
            .field("storage_backend", &self.storage_backend)
            .field("terminal", &self.terminal)
            .field("ui", &self.ui)
            .field("backend_commands", &self.backend_commands)
            .field("backend_executor", &"<shared backend executor>")
            .finish()
    }
}

impl Default for AppState {
    fn default() -> Self {
        let config = AppConfig::default();
        let mut storage = StorageManager::default();
        storage.app_config = config.clone();
        let ui = UiState::from_visual(&config.theme, &config.background);

        Self {
            config,
            sessions: SessionManager::default(),
            storage,
            storage_backend: None,
            terminal: TerminalManager::default(),
            ui,
            backend_commands: BackendCommandQueue::default(),
            backend_executor: noop_shared_backend_executor(),
        }
    }
}

impl AppState {
    /// 使用指定共享执行器替换默认占位执行器。
    pub fn with_backend_executor<E>(mut self, executor: E) -> Self
    where
        E: crate::backend::BackendExecutor + 'static,
    {
        self.backend_executor = shared_backend_executor(executor);
        self
    }

    /// 使用指定本地存储后端启用持久化。
    pub fn with_storage_backend(mut self, storage_backend: RedbStorage) -> Self {
        self.storage_backend = Some(storage_backend);
        self
    }

    /// 从已配置的本地存储后端保存当前持久化状态。
    pub fn persist_storage(&self) -> Result<(), StoragePersistenceError> {
        if let Some(storage_backend) = &self.storage_backend {
            storage_backend.save(&self.storage)?;
        }

        Ok(())
    }
}
