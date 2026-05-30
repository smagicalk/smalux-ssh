//! 核心应用根状态。
//!
//! 本文件不依赖 Slint。它描述应用“现在是什么状态”，以及状态持有的
//! 核心能力：配置、会话、存储、终端缓冲、UI 草稿、后端队列和后端执行器。
//! Slint、Web 或其他界面都应该把这里当作核心状态源，而不是直接绕过它操作
//! `SessionManager` 或 `StorageManager`。

use std::fmt;

use crate::backend::{
    BackendCommandQueue, SharedBackendExecutor, noop_shared_backend_executor,
    shared_backend_executor,
};
use crate::config::AppConfig;
use crate::session::SessionManager;
use crate::storage::{SqliteStorage, StorageManager, StoragePersistenceError};
use crate::terminal::TerminalManager;

use super::super::UiState;

/// 应用核心根状态。
///
/// 根状态只组合各个单一职责管理器，不直接实现 SSH、SFTP 或终端细节。
/// 行为入口集中在 `AppState::apply(Message)` 及其分发模块中，这样 UI 可以
/// 被替换，而核心行为仍然通过同一组消息和测试覆盖。
#[derive(Clone)]
pub struct AppState {
    /// 全局配置和运行偏好。
    ///
    /// 这里保存主题、背景、工作区偏好、安全偏好等可持久化配置。UI 只读取
    /// 经过 view model 处理后的展示值，不应直接假设配置内部字段如何渲染。
    pub config: AppConfig,
    /// SSH、SFTP、隧道和本地终端会话的运行态。
    ///
    /// 会话生命周期、连接状态、标签页归属和隧道状态都集中在这里，避免 UI
    /// 自己拼接“当前连接是否可用”这类规则。
    pub sessions: SessionManager,
    /// 内存中的持久化数据快照。
    ///
    /// 包含主机、分组、凭据元数据、Known Hosts、历史、片段、主题资料等。
    /// 修改后由 Adapter 判断是否需要调用 `persist_storage` 落盘。
    pub storage: StorageManager,
    /// 可选 SQLite 后端。
    ///
    /// 为 `None` 时应用仍可用内存存储运行，便于测试和无本地库的启动路径。
    pub storage_backend: Option<SqliteStorage>,
    /// 终端缓冲和本地终端入口。
    ///
    /// 只保存 UI 需要展示的终端状态；真实 PTY/SSH 句柄留在后端执行器。
    pub terminal: TerminalManager,
    /// 纯 UI 草稿状态。
    ///
    /// 这里保存输入框、弹窗、筛选、面板宽度等尚未提交到核心数据结构的值。
    /// 它仍属于核心状态树，是为了让非 Slint UI 也能复用同样交互流程。
    pub ui: UiState,
    /// 等待后端执行的命令队列。
    ///
    /// `apply(Message)` 只负责排队和更新同步状态；真正的网络/PTY 操作由
    /// `backend_executor` 在 pump 中异步执行。
    pub backend_commands: BackendCommandQueue,
    /// 后端执行器 Adapter。
    ///
    /// 测试默认使用 no-op 执行器；桌面启动时会替换为真实 SSH/PTY/SFTP 执行器。
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
    /// 将持久化配置里的工作区偏好同步到 UI 运行态。
    pub fn apply_workspace_preferences(&mut self) {
        self.ui.workspace.host_list_mode =
            host_list_mode_from_preference(self.config.workspace.host_list_mode);
        self.ui.workspace.language = language_from_preference(self.config.workspace.language);
        self.ui.workspace.theme =
            built_in_theme_from_preference(self.config.workspace.built_in_theme);
    }

    /// 使用指定共享执行器替换默认占位执行器。
    pub fn with_backend_executor<E>(mut self, executor: E) -> Self
    where
        E: crate::backend::BackendExecutor + 'static,
    {
        self.backend_executor = shared_backend_executor(executor);
        self
    }

    /// 使用指定本地存储后端启用持久化。
    pub fn with_storage_backend(mut self, storage_backend: SqliteStorage) -> Self {
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

fn host_list_mode_from_preference(
    preference: crate::config::HostListModePreference,
) -> crate::model::HostListMode {
    match preference {
        crate::config::HostListModePreference::Tree => crate::model::HostListMode::Tree,
        crate::config::HostListModePreference::Card => crate::model::HostListMode::Card,
    }
}

fn language_from_preference(
    preference: crate::config::LanguagePreference,
) -> crate::model::LanguageMode {
    crate::model::LanguageMode::from_preference(preference)
}

fn built_in_theme_from_preference(
    preference: crate::config::BuiltInThemePreference,
) -> crate::model::BuiltInTheme {
    crate::model::BuiltInTheme::from_preference(preference)
}
