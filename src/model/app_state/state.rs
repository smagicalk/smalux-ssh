//! 过渡期应用组合状态。
//!
//! 本文件不依赖 Slint，但它组合了无 GUI 核心状态和当前桌面 UI 草稿。
//! 新 UI 应优先从 `CoreState` 开始；这个类型主要服务旧测试和迁移期兼容。

use std::fmt;
use std::ops::{Deref, DerefMut};

use super::super::UiState;
use crate::storage::StoragePersistenceError;

/// 旧桌面过渡根状态。
///
/// 真正与具体 GUI 无关的数据在 `CoreState`。这个类型额外组合 `UiState`，
/// 用来承载当前桌面 UI 的输入框、弹窗和筛选草稿。桌面主路径已经改用
/// `DesktopAppState`，这里主要保留给旧测试和迁移期模块。
#[derive(Clone)]
pub struct AppState {
    /// 与具体 GUI 无关的核心运行态。
    pub core: crate::core::CoreState,
    /// 纯 UI 草稿状态。
    ///
    /// 这里保存输入框、弹窗、筛选、面板宽度等尚未提交到核心数据结构的值。
    /// 它属于当前桌面 Adapter，不属于无 GUI 核心。
    pub ui: UiState,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("core", &self.core)
            .field("ui", &self.ui)
            .finish()
    }
}

impl Deref for AppState {
    type Target = crate::core::CoreState;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl DerefMut for AppState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}

impl Default for AppState {
    fn default() -> Self {
        let core = crate::core::CoreState::default();
        let ui = UiState::from_visual(&core.config.theme, &core.config.background);

        Self { core, ui }
    }
}

impl AppState {
    /// 将持久化配置里的工作区偏好同步到 UI 运行态。
    pub fn apply_workspace_preferences(&mut self) {
        self.ui
            .apply_workspace_preferences_from_config(&self.core.config);
    }

    /// 从已配置的本地存储后端保存当前持久化状态。
    pub fn persist_storage(&self) -> Result<(), StoragePersistenceError> {
        self.core.persist_storage()
    }

    /// 使用指定共享执行器替换默认占位执行器。
    pub fn with_backend_executor<E>(mut self, executor: E) -> Self
    where
        E: crate::backend::BackendExecutor + 'static,
    {
        self.core = self.core.with_backend_executor(executor);
        self
    }

    /// 使用指定本地存储后端启用持久化。
    pub fn with_storage_backend(mut self, storage_backend: crate::storage::SqliteStorage) -> Self {
        self.core = self.core.with_storage_backend(storage_backend);
        self
    }
}
