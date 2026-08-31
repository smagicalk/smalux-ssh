use std::sync::{Arc, RwLock};
use crate::app_hook::{AppGlobalHookEngine, AutoConfigBackupHook};
use crate::domain::{ActivityBarRegistry, NavigationRequest, NavigationRouter};
use crate::hook::HookEngine;
use crate::storage::{AppStorage, MockStorage};

/// 无界面依赖的核心状态引擎 (Core State Engine)
///
/// 内部封装聚合存储门面 (`Arc<dyn AppStorage>`)、终端 Hook 调度中心 (`Arc<HookEngine>`)、
/// 应用级全局 Hook 引擎 (`Arc<AppGlobalHookEngine>`)、侧边栏动态注册中心 (`Arc<ActivityBarRegistry>`)
/// 以及全局统一导航路由中枢 (`Arc<RwLock<NavigationRouter>>`)。
#[derive(Clone)]
pub struct CoreState {
    storage: Arc<dyn AppStorage>,
    hooks: Arc<HookEngine>,
    app_hooks: Arc<AppGlobalHookEngine>,
    activity_bar: Arc<ActivityBarRegistry>,
    navigation: Arc<RwLock<NavigationRouter>>,
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

        let activity_bar = Arc::new(ActivityBarRegistry::new_with_defaults());
        let navigation = Arc::new(RwLock::new(NavigationRouter::default()));

        Self {
            storage,
            hooks: Arc::new(engine),
            app_hooks: Arc::new(app_engine),
            activity_bar,
            navigation,
        }
    }

    /// 使用任意存储后端创建 CoreState (自动挂载内置 Hook 插件)
    pub fn new(storage: Arc<dyn AppStorage>) -> Self {
        let engine = HookEngine::new();
        engine.register(Arc::new(crate::hook::DangerousCommandGuard::new()));
        engine.register(Arc::new(crate::hook::SessionAuditLogger::new()));
        engine.register(Arc::new(crate::hook::HistoryTrackingHook::new(Arc::clone(&storage))));

        let app_engine = AppGlobalHookEngine::new();
        app_engine.register(Arc::new(AutoConfigBackupHook::new()));

        let activity_bar = Arc::new(ActivityBarRegistry::new_with_defaults());
        let navigation = Arc::new(RwLock::new(NavigationRouter::default()));

        Self {
            storage,
            hooks: Arc::new(engine),
            app_hooks: Arc::new(app_engine),
            activity_bar,
            navigation,
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

    /// 获取侧边栏动态注册中心引用
    pub fn activity_bar(&self) -> &Arc<ActivityBarRegistry> {
        &self.activity_bar
    }

    /// 获取导航路由器引用
    pub fn navigation(&self) -> &Arc<RwLock<NavigationRouter>> {
        &self.navigation
    }

    /// 统一发起页面跳转导航，并自动触发生命周期 Hook
    pub fn navigate_to(&self, request: NavigationRequest) {
        let (prev, curr) = {
            let mut nav = self.navigation.write().unwrap();
            nav.navigate_to(request)
        };

        // 1. 若有上一个激活的模块，触发其失活生命周期
        if let Some(p) = prev {
            self.app_hooks.dispatch_module_deactivated(&p.target_tab);
        }

        // 2. 广播全局导航请求
        self.app_hooks.dispatch_navigation_requested(&curr);

        // 3. 触发目标模块激活生命周期
        self.app_hooks.dispatch_module_activated(
            &curr.target_tab,
            curr.sub_section.as_deref(),
            &curr.params,
        );
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
