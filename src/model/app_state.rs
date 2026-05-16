//! 应用根状态和消息调度。

use std::fmt;

use crate::backend::{
    BackendCommandQueue, SharedBackendExecutor, noop_shared_backend_executor,
    shared_backend_executor,
};
use crate::config::AppConfig;
use crate::session::SessionManager;
use crate::storage::{RedbStorage, StorageManager, StoragePersistenceError};
use crate::terminal::TerminalManager;

use super::UiState;

#[cfg(test)]
use super::{HostId, SnippetId, VisualSettingsDraftField, WorkspacePage};

mod backend_events;
mod backend_pump;
#[cfg(test)]
mod backend_pump_tests;
mod dispatch;
mod launch;
mod launch_remote_command;
mod launch_sftp;
mod launch_sftp_transfer;
#[cfg(test)]
mod launch_tests;
mod launch_tunnel;
mod message;
mod session_tabs;
mod snippets;
#[cfg(test)]
mod snippets_tests;
mod storage_admin;
#[cfg(test)]
mod tests;
mod ui_drafts;
#[cfg(test)]
mod ui_drafts_tests;
mod visual_settings;
#[cfg(test)]
mod visual_settings_tests;
mod workspace;
mod workspace_ui;

pub use message::Message;

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

/// 应用消息处理结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppUpdateOutcome {
    pub state_changed: bool,
    pub queued_backend_commands: usize,
    pub executed_backend_commands: usize,
    pub applied_backend_events: usize,
    pub error: Option<String>,
}

impl AppUpdateOutcome {
    /// 是否有状态变化或错误反馈。
    pub fn changed(&self) -> bool {
        self.state_changed || self.error.is_some()
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
