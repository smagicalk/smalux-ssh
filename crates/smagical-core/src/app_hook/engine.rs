//! 全局应用级 Hook 调度引擎与动态热插拔监听者管理器。

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::hook::HookDecision;
use super::builtin::FunctionalGlobalHook;
use super::traits::AppGlobalHook;
use super::types::{AppBootContext, AppExitContext, ConfigChangeEvent, WindowState};

/// 动态监听订阅句柄 (用于热拔除/取消注册)。
pub struct ListenerHandle {
    id: u64,
    engine: std::sync::Weak<AppGlobalHookEngine>,
}

impl ListenerHandle {
    /// 取消当前监听订阅并从调度引擎中热拔除。
    pub fn dispose(self) {
        if let Some(engine) = self.engine.upgrade() {
            engine.unregister_by_id(self.id);
        }
    }
}

/// 内部注册项包装
struct HookEntry {
    id: u64,
    hook: Arc<dyn AppGlobalHook>,
}

/// 全局应用级 Hook 调度与事件分发引擎。
///
/// 具备多监听者动态热插拔注册、优先级自动排序以及物理级 Panic 隔离保护。
pub struct AppGlobalHookEngine {
    hooks: RwLock<Vec<HookEntry>>,
    next_id: AtomicU64,
}

impl Default for AppGlobalHookEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AppGlobalHookEngine {
    /// 创建一个新的全局 Hook 引擎实例。
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
            next_id: AtomicU64::new(1),
        }
    }

    // =========================================================================
    // 1. 动态热插拔注册与注销 (Hot-Plugging & Lifecycle)
    // =========================================================================

    /// 【动态注册 Hook 插件】：按优先级自动降序排序，返回唯一注册 ID。
    pub fn register(&self, hook: Arc<dyn AppGlobalHook>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut list = self.hooks.write().unwrap();
        list.push(HookEntry { id, hook });
        // 按优先级降序排序 (数值越大越先执行)
        list.sort_by_key(|b| std::cmp::Reverse(b.hook.priority()));
        id

    }

    /// 【动态注销 Hook 插件】：根据插件名称从引擎中卸载。
    pub fn unregister(&self, name: &str) {
        let mut list = self.hooks.write().unwrap();
        list.retain(|entry| entry.hook.name() != name);
    }

    /// 【根据 ID 注销 Hook 插件】。
    pub fn unregister_by_id(&self, id: u64) {
        let mut list = self.hooks.write().unwrap();
        list.retain(|entry| entry.id != id);
    }

    /// 获取当前已注册的有效 Hook 插件数量。
    pub fn len(&self) -> usize {
        self.hooks.read().unwrap().len()
    }

    /// 当前注册列表是否为空。
    pub fn is_empty(&self) -> bool {
        self.hooks.read().unwrap().is_empty()
    }

    // =========================================================================
    // 2. 函数式闭包一键订阅 (Functional Shortcuts)
    // =========================================================================

    /// 一键挂载参数变动闭包监听器。
    pub fn on_config_changed<F>(&self, callback: F) -> u64
    where
        F: Fn(&ConfigChangeEvent) + Send + Sync + 'static,
    {
        self.register(Arc::new(FunctionalGlobalHook::for_config_changed(
            "anonymous_config_listener",
            callback,
        )))
    }

    /// 一键挂载左侧菜单切换闭包监听器。
    pub fn on_left_menu_clicked<F>(&self, callback: F) -> u64
    where
        F: Fn(&str, &str) + Send + Sync + 'static,
    {
        self.register(Arc::new(FunctionalGlobalHook::for_left_menu(
            "anonymous_left_menu_listener",
            callback,
        )))
    }

    /// 一键挂载应用退出前拦截闭包监听器。
    pub fn on_app_before_exit<F>(&self, priority: i32, callback: F) -> u64
    where
        F: Fn(&AppExitContext) -> HookDecision + Send + Sync + 'static,
    {
        self.register(Arc::new(FunctionalGlobalHook::for_before_exit(
            "anonymous_before_exit_guard",
            priority,
            callback,
        )))
    }

    // =========================================================================
    // 3. 全生命周期事件广播分发 (内置 Panic 物理隔离)
    // =========================================================================

    /// 广播应用启动引导事件 (`on_app_boot`)。
    pub fn dispatch_app_boot(&self, ctx: &AppBootContext) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ctx_cloned = ctx.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_app_boot(&ctx_cloned);
            }));
        }
    }

    /// 广播主界面就绪事件 (`on_app_ready`)。
    pub fn dispatch_app_ready(&self) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_app_ready();
            }));
        }
    }

    /// 广播左侧活动栏菜单点击事件 (`on_left_menu_clicked`)。
    pub fn dispatch_left_menu_clicked(&self, menu_id: &str, old_menu_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let m_id = menu_id.to_string();
            let old_id = old_menu_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_left_menu_clicked(&m_id, &old_id);
            }));
        }
    }

    /// 广播主工作区视图流转事件 (`on_main_view_switched`)。
    pub fn dispatch_main_view_switched(&self, current_view: &str, previous_view: &str) {

        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let cur = current_view.to_string();
            let prev = previous_view.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_main_view_switched(&cur, &prev);
            }));
        }
    }

    /// 广播深浅色外观切换事件 (`on_theme_mode_toggled`)。
    pub fn dispatch_theme_mode_toggled(&self, is_dark: bool) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_theme_mode_toggled(is_dark);
            }));
        }
    }

    /// 广播全局快捷指令执行事件 (`on_command_executed`)。
    pub fn dispatch_command_executed(&self, command_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let cmd = command_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_command_executed(&cmd);
            }));
        }
    }

    /// 广播全局参数变更事件 (`on_global_config_changed`)。
    pub fn dispatch_config_changed(&self, event: &ConfigChangeEvent) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ev = event.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_global_config_changed(&ev);
            }));
        }
    }

    /// 广播窗口全局状态变动事件 (`on_window_state_changed`)。
    pub fn dispatch_window_state_changed(&self, state: WindowState) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_window_state_changed(state);
            }));
        }
    }

    /// 广播应用退出前拦截询问 (`on_app_before_exit`)。
    ///
    /// 若任一 Hook 返回 `HookDecision::Abort`，则立刻中断退出流程。
    pub fn dispatch_app_before_exit(&self, ctx: &AppExitContext) -> HookDecision {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ctx_cloned = ctx.clone();
            let res = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_app_before_exit(&ctx_cloned)
            }));
            if let Ok(decision) = res
                && matches!(decision, HookDecision::Abort { .. })
            {
                return decision;
            }
        }
        HookDecision::Continue

    }

    /// 广播应用完全退出事件 (`on_app_exit`)。
    pub fn dispatch_app_exit(&self, ctx: &AppExitContext) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ctx_cloned = ctx.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_app_exit(&ctx_cloned);
            }));
        }
    }
}
