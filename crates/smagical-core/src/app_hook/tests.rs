//! 全局应用 Hook 调度引擎单元测试。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::hook::HookDecision;
use super::builtin::AutoConfigBackupHook;
use super::engine::AppGlobalHookEngine;
use super::traits::AppGlobalHook;
use super::types::{AppBootContext, AppExitContext, ConfigChangeEvent, WindowState};


/// 测试自定义测试插件
struct MockAppPlugin {
    name: &'static str,
    priority: i32,
    boot_called: AtomicBool,
    config_changes_count: AtomicUsize,
    last_menu: std::sync::RwLock<String>,
}

impl MockAppPlugin {
    fn new(name: &'static str, priority: i32) -> Self {
        Self {
            name,
            priority,
            boot_called: AtomicBool::new(false),
            config_changes_count: AtomicUsize::new(0),
            last_menu: std::sync::RwLock::new(String::new()),
        }
    }
}

impl AppGlobalHook for MockAppPlugin {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn on_app_boot(&self, _ctx: &AppBootContext) {
        self.boot_called.store(true, Ordering::SeqCst);
    }

    fn on_global_config_changed(&self, _event: &ConfigChangeEvent) {
        self.config_changes_count.fetch_add(1, Ordering::SeqCst);
    }

    fn on_left_menu_clicked(&self, menu_id: &str, _old_menu_id: &str) {
        let mut m = self.last_menu.write().unwrap();
        *m = menu_id.to_string();
    }
}

#[test]
fn test_dynamic_register_and_unregister() {
    let engine = AppGlobalHookEngine::new();
    assert_eq!(engine.len(), 0);

    let p1 = Arc::new(MockAppPlugin::new("plugin_1", 10));
    let p2 = Arc::new(MockAppPlugin::new("plugin_2", 20));

    let _id1 = engine.register(Arc::clone(&p1) as Arc<dyn AppGlobalHook>);
    let id2 = engine.register(Arc::clone(&p2) as Arc<dyn AppGlobalHook>);

    assert_eq!(engine.len(), 2);

    // 根据名称注销
    engine.unregister("plugin_1");
    assert_eq!(engine.len(), 1);

    // 根据 ID 注销
    engine.unregister_by_id(id2);
    assert_eq!(engine.len(), 0);
    assert!(engine.is_empty());
}

#[test]
fn test_config_change_dispatch_and_builtin_backup() {
    let engine = AppGlobalHookEngine::new();
    let backup_hook = Arc::new(AutoConfigBackupHook::new());
    engine.register(Arc::clone(&backup_hook) as Arc<dyn AppGlobalHook>);

    assert_eq!(backup_hook.backup_count(), 0);

    let event = ConfigChangeEvent::new(
        "appearance.theme",
        "builtin.ui.darcula",
        "builtin.ui.monokai",
        "command_palette",
    );
    engine.dispatch_config_changed(&event);

    assert_eq!(backup_hook.backup_count(), 1);
    assert!(event.affects("appearance"));
    assert!(event.affects("appearance.theme"));
    assert!(!event.affects("terminal"));
    assert!(event.is_changed());

    let event_unchanged = ConfigChangeEvent::new("terminal.font_size", "14", "14", "ui");
    assert!(!event_unchanged.is_changed());
    assert!(event_unchanged.affects("terminal"));

    let event2 = ConfigChangeEvent::new("terminal.font_size", "14", "16", "settings_drawer");
    engine.dispatch_config_changed(&event2);

    assert_eq!(backup_hook.backup_count(), 2);
}


#[test]
fn test_functional_closure_listeners() {
    let engine = AppGlobalHookEngine::new();
    let captured_key = Arc::new(std::sync::RwLock::new(String::new()));
    let captured_key_clone = Arc::clone(&captured_key);

    let _handle_id = engine.on_config_changed(move |ev| {
        let mut k = captured_key_clone.write().unwrap();
        *k = ev.key.clone();
    });

    let event = ConfigChangeEvent::new("ssh.keepalive", "60", "30", "ui");
    engine.dispatch_config_changed(&event);

    assert_eq!(*captured_key.read().unwrap(), "ssh.keepalive");
}

#[test]
fn test_before_exit_guard_interception() {
    let engine = AppGlobalHookEngine::new();

    // 挂载退出拦截保护
    let _guard_id = engine.on_app_before_exit(100, |ctx| {
        if ctx.active_sessions_count > 0 {
            HookDecision::Abort {
                reason: "仍有活跃终端会话未关闭".to_string(),
            }
        } else {
            HookDecision::Continue
        }
    });

    // 场景 A: 有 3 个活跃会话 -> 应该被拦截
    let ctx_busy = AppExitContext::normal(3);
    let decision = engine.dispatch_app_before_exit(&ctx_busy);
    assert!(matches!(decision, HookDecision::Abort { .. }));

    // 场景 B: 0 个活跃会话 -> 允许安全退出
    let ctx_idle = AppExitContext::normal(0);
    let decision2 = engine.dispatch_app_before_exit(&ctx_idle);
    assert!(matches!(decision2, HookDecision::Continue));
}

#[test]
fn test_panic_safety_isolation() {
    struct PanicPlugin;
    impl AppGlobalHook for PanicPlugin {
        fn name(&self) -> &'static str {
            "panic_plugin"
        }
        fn on_global_config_changed(&self, _event: &ConfigChangeEvent) {
            panic!("第三方外挂插件故意崩溃抛出异常！");
        }
    }

    let engine = AppGlobalHookEngine::new();
    engine.register(Arc::new(PanicPlugin));

    let event = ConfigChangeEvent::new("test.key", "1", "2", "test");
    // 调度引擎不会因为某个插件 panic 而崩溃，安全继续执行
    engine.dispatch_config_changed(&event);
}

#[test]
fn test_shell_navigation_and_window_state_dispatch() {
    let engine = AppGlobalHookEngine::new();
    let plugin = Arc::new(MockAppPlugin::new("mock_nav", 0));
    engine.register(Arc::clone(&plugin) as Arc<dyn AppGlobalHook>);

    engine.dispatch_left_menu_clicked("history", "hosts");
    assert_eq!(*plugin.last_menu.read().unwrap(), "history");

    engine.dispatch_window_state_changed(WindowState::Minimized);
}
