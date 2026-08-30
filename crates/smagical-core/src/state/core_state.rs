use std::sync::Arc;
use crate::hook::HookEngine;
use crate::storage::{AppStorage, MockStorage};


/// 无界面依赖的核心状态引擎 (Core State Engine)
///
/// 内部封装聚合存储门面 (`Arc<dyn AppStorage>`) 与全局 Hook 调度中心 (`Arc<HookEngine>`)。
#[derive(Clone)]
pub struct CoreState {
    storage: Arc<dyn AppStorage>,
    hooks: Arc<HookEngine>,
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
        Self::with_hooks(storage, Arc::new(engine))
    }

    /// 使用任意存储后端创建 CoreState (自动挂载内置 Hook 插件)
    pub fn new(storage: Arc<dyn AppStorage>) -> Self {
        let engine = HookEngine::new();
        engine.register(Arc::new(crate::hook::DangerousCommandGuard::new()));
        engine.register(Arc::new(crate::hook::SessionAuditLogger::new()));
        engine.register(Arc::new(crate::hook::HistoryTrackingHook::new(Arc::clone(&storage))));
        Self::with_hooks(storage, Arc::new(engine))
    }


    /// 使用指定存储后端与自定义 HookEngine 创建 CoreState
    pub fn with_hooks(storage: Arc<dyn AppStorage>, hooks: Arc<HookEngine>) -> Self {
        Self { storage, hooks }
    }

    /// 获取底层存储门面引用
    pub fn storage(&self) -> &Arc<dyn AppStorage> {
        &self.storage
    }

    /// 获取全局 Hook 调度引擎引用
    pub fn hooks(&self) -> &Arc<HookEngine> {
        &self.hooks
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
