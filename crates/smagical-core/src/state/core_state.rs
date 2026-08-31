use std::sync::Arc;
use crate::app_hook::{AppGlobalHookEngine, AutoConfigBackupHook};
use crate::hook::HookEngine;
use crate::storage::{AppStorage, MockStorage};

/// 无界面依赖的核心状态引擎 (Core State Engine)
///
/// 内部封装聚合存储门面 (`Arc<dyn AppStorage>`)、终端 Hook 调度中心 (`Arc<HookEngine>`) 以及应用级全局 Hook 引擎 (`Arc<AppGlobalHookEngine>`)。
#[derive(Clone)]
pub struct CoreState {
    storage: Arc<dyn AppStorage>,
    hooks: Arc<HookEngine>,
    app_hooks: Arc<AppGlobalHookEngine>,
}

impl CoreState {
    /// 默认采用预设种子内存存储与内置 Hook 插件创建 CoreState
    pub fn new_mock() -> Self {
        tracing::debug!(target: "smagical_core", "初始化 CoreState 核心状态引擎 (MockStorage 模式)");
        let storage: Arc<dyn AppStorage> = Arc::new(MockStorage::new_seeded());
        let engine = HookEngine::new();
        engine.register(Arc::new(crate::hook::DangerousCommandGuard::new()));
        engine.register(Arc::new(crate::hook::SessionAuditLogger::new()));
        engine.register(Arc::new(crate::hook::HistoryTrackingHook::new(Arc::clone(&storage))));

        let app_engine = AppGlobalHookEngine::new();
        app_engine.register(Arc::new(AutoConfigBackupHook::new()));

        Self::with_all(storage, Arc::new(engine), Arc::new(app_engine))
    }

    /// 使用任意存储后端创建 CoreState (自动挂载内置 Hook 插件)
    pub fn new(storage: Arc<dyn AppStorage>) -> Self {
        let engine = HookEngine::new();
        engine.register(Arc::new(crate::hook::DangerousCommandGuard::new()));
        engine.register(Arc::new(crate::hook::SessionAuditLogger::new()));
        engine.register(Arc::new(crate::hook::HistoryTrackingHook::new(Arc::clone(&storage))));

        let app_engine = AppGlobalHookEngine::new();
        app_engine.register(Arc::new(AutoConfigBackupHook::new()));

        Self::with_all(storage, Arc::new(engine), Arc::new(app_engine))
    }

    /// 使用指定存储后端与自定义 HookEngine 创建 CoreState
    pub fn with_hooks(storage: Arc<dyn AppStorage>, hooks: Arc<HookEngine>) -> Self {
        let app_engine = AppGlobalHookEngine::new();
        app_engine.register(Arc::new(AutoConfigBackupHook::new()));
        Self::with_all(storage, hooks, Arc::new(app_engine))
    }

    /// 使用完整自定义组件创建 CoreState
    pub fn with_all(
        storage: Arc<dyn AppStorage>,
        hooks: Arc<HookEngine>,
        app_hooks: Arc<AppGlobalHookEngine>,
    ) -> Self {
        Self {
            storage,
            hooks,
            app_hooks,
        }
    }

    /// 获取底层存储门面引用
    pub fn storage(&self) -> &Arc<dyn AppStorage> {
        &self.storage
    }

    /// 获取终端会话 Hook 调度引擎引用
    pub fn hooks(&self) -> &Arc<HookEngine> {
        &self.hooks
    }

    /// 获取全局应用级 Hook 调度引擎引用
    pub fn app_hooks(&self) -> &Arc<AppGlobalHookEngine> {
        &self.app_hooks
    }
}



impl Default for CoreState {
    fn default() -> Self {
        Self::new_mock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_mock_state_has_seeded_groups_and_hosts() {
        let state = CoreState::new_mock();
        let groups = state.storage().groups().list_all().unwrap();
        let hosts = state.storage().hosts().list_all().unwrap();

        assert!(!groups.is_empty());
        assert!(!hosts.is_empty());
        assert_eq!(groups[0].id, "grp-prod");
    }
}
