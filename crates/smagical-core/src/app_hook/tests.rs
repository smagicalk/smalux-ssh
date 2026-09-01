//! 全局应用 Hook 调度引擎单元测试。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::hook::HookDecision;
use super::builtin::{AutoConfigBackupHook, FunctionalGlobalHook};
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

    fn on_config_changed(&self, _event: &ConfigChangeEvent) {
        self.config_changes_count.fetch_add(1, Ordering::SeqCst);
    }

    fn on_shell_left_menu_clicked(&self, menu_id: &str, _old_menu_id: &str) {
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
fn test_raii_handle_drop_and_detach() {
    let engine = Arc::new(AppGlobalHookEngine::new());
    let p1 = Arc::new(MockAppPlugin::new("p1", 0));
    let p2 = Arc::new(MockAppPlugin::new("p2", 0));

    // 1. 注册并离开作用域，handle drop 自动注销
    {
        let _handle = engine.register_with_handle(Arc::clone(&p1) as Arc<dyn AppGlobalHook>);
        assert_eq!(engine.len(), 1);
    }
    assert_eq!(engine.len(), 0);

    // 2. 注册并 detach，handle drop 不会注销
    {
        let handle = engine.register_with_handle(Arc::clone(&p2) as Arc<dyn AppGlobalHook>);
        assert_eq!(engine.len(), 1);
        handle.detach();
    }
    assert_eq!(engine.len(), 1);
}


#[test]
fn test_config_change_dispatch_and_builtin_backup() {
    let engine = AppGlobalHookEngine::new();
    let backup_hook = Arc::new(AutoConfigBackupHook::new());
    let mock_plugin = Arc::new(MockAppPlugin::new("mock_plugin", 10));

    engine.register(Arc::clone(&backup_hook) as Arc<dyn AppGlobalHook>);
    engine.register(Arc::clone(&mock_plugin) as Arc<dyn AppGlobalHook>);

    let event = ConfigChangeEvent::new("terminal.font_size", "14", "16", "user_settings");
    engine.dispatch_config_changed(&event);

    assert_eq!(backup_hook.backup_count(), 1);
    assert_eq!(mock_plugin.config_changes_count.load(Ordering::SeqCst), 1);

    let event2 = ConfigChangeEvent::new("terminal.cursor_blink", "true", "false", "theme_preset");
    engine.dispatch_config_changed(&event2);

    assert_eq!(backup_hook.backup_count(), 2);
    assert_eq!(mock_plugin.config_changes_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_functional_closure_listeners() {
    let engine = AppGlobalHookEngine::new();
    let count = Arc::new(AtomicUsize::new(0));
    let count_cloned = Arc::clone(&count);

    let hook = FunctionalGlobalHook::for_config_changed("closure_hook", move |_e| {
        count_cloned.fetch_add(1, Ordering::SeqCst);
    });

    engine.register(Arc::new(hook));
    let event = ConfigChangeEvent::new("appearance.theme", "dark", "light", "user");
    engine.dispatch_config_changed(&event);

    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_before_exit_guard_interception() {
    struct SessionExitGuard;
    impl AppGlobalHook for SessionExitGuard {
        fn name(&self) -> &'static str {
            "session_exit_guard"
        }
        fn on_app_before_exit(&self, ctx: &AppExitContext) -> HookDecision {
            if ctx.active_sessions_count > 0 {
                HookDecision::Abort {
                    reason: "还有活跃会话未保存！".into(),
                }
            } else {
                HookDecision::Continue
            }
        }
    }

    let engine = AppGlobalHookEngine::new();
    engine.register(Arc::new(SessionExitGuard));

    // 场景 A: 存在 2 个活跃会话 -> 必须拦截
    let ctx_busy = AppExitContext::normal(2);
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
        fn on_config_changed(&self, _event: &ConfigChangeEvent) {
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

    engine.dispatch_shell_left_menu_clicked("history", "hosts");
    assert_eq!(*plugin.last_menu.read().unwrap(), "history");

    engine.dispatch_shell_window_state_changed(WindowState::Minimized);
}

#[test]
fn test_three_column_host_hooks_flow() {
    use crate::domain::{ActiveTerminalSessionContext, HostRecord, TerminalAction};

    struct ThreeColumnObserver {
        asset_created_count: AtomicUsize,
        last_focus_session: std::sync::RwLock<Option<String>>,
        right_panel_open: AtomicBool,
        last_action: std::sync::RwLock<Option<String>>,
    }

    impl AppGlobalHook for ThreeColumnObserver {
        fn name(&self) -> &'static str {
            "three_column_observer"
        }

        fn on_host_asset_created(&self, _host: &HostRecord) {
            self.asset_created_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_host_terminal_focus_changed(&self, ctx: Option<&ActiveTerminalSessionContext>) {
            let mut s = self.last_focus_session.write().unwrap();
            *s = ctx.map(|c| c.session_id.clone());
        }

        fn on_host_right_drawer_toggled(&self, is_open: bool, _active_panel_id: &str) {
            self.right_panel_open.store(is_open, Ordering::SeqCst);
        }

        fn on_host_terminal_action_requested(&self, _session_id: &str, action: &TerminalAction) {
            let mut a = self.last_action.write().unwrap();
            if let TerminalAction::ExecuteCommand(cmd) = action {
                *a = Some(cmd.clone());
            }
        }
    }


    let engine = AppGlobalHookEngine::new();
    let observer = Arc::new(ThreeColumnObserver {
        asset_created_count: AtomicUsize::new(0),
        last_focus_session: std::sync::RwLock::new(None),
        right_panel_open: AtomicBool::new(false),
        last_action: std::sync::RwLock::new(None),
    });
    engine.register(Arc::clone(&observer) as Arc<dyn AppGlobalHook>);

    // 1. 左栏：新建主机资产
    let host = HostRecord::new("h-1", "Prod-Server", "192.168.1.100", 22);
    engine.dispatch_host_asset_created(&host);
    assert_eq!(observer.asset_created_count.load(Ordering::SeqCst), 1);

    // 2. 中栏：终端聚焦
    let ctx = ActiveTerminalSessionContext::ssh(
        "term-sess-99",
        "h-1",
        "Prod-Server",
        "192.168.1.100",
        22,
        "root",
        "生产集群",
        vec!["prod".into()],
    );
    engine.dispatch_host_terminal_focus_changed(Some(&ctx));
    assert_eq!(observer.last_focus_session.read().unwrap().as_deref(), Some("term-sess-99"));


    // 3. 右栏：展开抽屉并注入动作
    engine.dispatch_host_right_drawer_toggled(true, "snippets");
    assert!(observer.right_panel_open.load(Ordering::SeqCst));

    engine.dispatch_host_terminal_action_requested("term-sess-99", &TerminalAction::ExecuteCommand("tail -f app.log".into()));
    assert_eq!(observer.last_action.read().unwrap().as_deref(), Some("tail -f app.log"));
}

#[test]
fn test_dangerous_file_guard_interception() {
    use super::builtin::DangerousFileGuardHook;

    let engine = AppGlobalHookEngine::new();
    engine.register(Arc::new(DangerousFileGuardHook::new()));

    // 1. 尝试删除系统敏感路径 -> 应被拦截
    let blocked_paths = vec![
        "/",
        "/etc",
        "/bin",
        "/usr",
        "C:\\Windows",
        "c:/windows/system32",
        "C:\\Program Files",
        "c:",
        "d:/",
    ];

    for path in blocked_paths {
        let decision = engine.dispatch_file_operation_before("delete", false, path);
        if let HookDecision::Abort { reason } = decision {
            assert!(reason.contains("安全守护拦截"), "路径 [{}] 应被拦截，实际原因: {}", path, reason);
        } else {
            panic!("路径 [{}] 应被高危拦截，但实际通过了: {:?}", path, decision);
        }
    }

    // 2. 正常业务路径删除 -> 应放行
    let safe_paths = vec![
        "/home/developer/workspace/project.tar.gz",
        "/var/log/nginx/access.log",
        "F:/code/rust/smalux-ssh/target/debug/test.txt",
        "C:\\Users\\dev\\Documents\\notes.md",
    ];

    for path in safe_paths {
        let decision = engine.dispatch_file_operation_before("delete", false, path);
        assert_eq!(decision, HookDecision::Continue, "安全路径 [{}] 应被放行", path);
    }
}

#[test]
fn test_file_domain_lifecycle_flow() {
    use crate::domain::{TransferDirection, TransferStatus, TransferTask};

    struct FileDomainObserver {
        last_opened_tab: std::sync::RwLock<Option<(String, String)>>,
        last_navigated: std::sync::RwLock<Option<(String, String)>>,
        transfer_started_count: AtomicUsize,
        transfer_completed_count: AtomicUsize,
    }

    impl AppGlobalHook for FileDomainObserver {
        fn name(&self) -> &'static str {
            "file_domain_observer"
        }

        fn on_file_tab_opened(&self, session_id: &str, host_id: &str, _initial_path: &str) {
            let mut o = self.last_opened_tab.write().unwrap();
            *o = Some((session_id.to_string(), host_id.to_string()));
        }

        fn on_file_tab_navigated(&self, _session_id: &str, _is_remote: bool, from_path: &str, to_path: &str) {
            let mut n = self.last_navigated.write().unwrap();
            *n = Some((from_path.to_string(), to_path.to_string()));
        }

        fn on_file_transfer_started(&self, _task_id: &str) {
            self.transfer_started_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_file_transfer_completed(&self, _task: &TransferTask) {
            self.transfer_completed_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    let engine = AppGlobalHookEngine::new();
    let observer = Arc::new(FileDomainObserver {
        last_opened_tab: std::sync::RwLock::new(None),
        last_navigated: std::sync::RwLock::new(None),
        transfer_started_count: AtomicUsize::new(0),
        transfer_completed_count: AtomicUsize::new(0),
    });
    engine.register(Arc::clone(&observer) as Arc<dyn AppGlobalHook>);

    // 1. 会话建立
    engine.dispatch_file_tab_opened("rtab-1", "host-k8s-01", "/root");
    assert_eq!(
        *observer.last_opened_tab.read().unwrap(),
        Some(("rtab-1".into(), "host-k8s-01".into()))
    );

    // 2. 路径跳转
    engine.dispatch_file_tab_navigated("rtab-1", true, "/root", "/root/src");
    assert_eq!(
        *observer.last_navigated.read().unwrap(),
        Some(("/root".into(), "/root/src".into()))
    );

    // 3. 传输管道流转
    let task = TransferTask {
        id: "task-001".into(),
        parent_id: None,
        session_id: "rtab-1".into(),
        filename: "app.bin".into(),
        is_dir: false,
        is_expanded: false,
        level: 0,
        item_count_text: "".into(),
        source_path: "F:/code/app.bin".into(),
        target_path: "/root/app.bin".into(),
        direction: TransferDirection::Upload,
        total_bytes: 10_000,
        transferred_bytes: 10_000,
        speed_bytes_per_sec: 1_000_000,
        status: TransferStatus::Completed,
        error_message: None,
    };

    engine.dispatch_file_transfer_started("task-001");
    assert_eq!(observer.transfer_started_count.load(Ordering::SeqCst), 1);

    engine.dispatch_file_transfer_completed(&task);
    assert_eq!(observer.transfer_completed_count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_host_audit_logger_file_domain_coverage() {
    use super::builtin::HostAuditLogHook;
    use crate::domain::{TransferDirection, TransferStatus, TransferTask};

    let engine = AppGlobalHookEngine::new();
    engine.register(Arc::new(HostAuditLogHook::new()));

    // 验证所有 file 域 dispatch 方法在挂载 HostAuditLogHook 时均能正常执行且不发生 panic
    let _ = engine.dispatch_file_tab_opening("host-1", "/root");
    engine.dispatch_file_tab_opened("rtab-1", "host-1", "/root");
    engine.dispatch_file_tab_focus_changed(Some("rtab-1"), true, "/root");
    engine.dispatch_file_tab_navigated("rtab-1", true, "/root", "/root/build");
    engine.dispatch_file_tab_closed("rtab-1");

    let _ = engine.dispatch_file_operation_before("create", false, "/home/user/new.txt");
    engine.dispatch_file_operation_completed("create", false, "/home/user/new.txt", true);
    engine.dispatch_file_operation_completed("delete", false, "/home/user/bad.txt", false);

    let task = TransferTask {
        id: "task-002".into(),
        parent_id: None,
        session_id: "rtab-1".into(),
        filename: "test.iso".into(),
        is_dir: false,
        is_expanded: false,
        level: 0,
        item_count_text: "".into(),
        source_path: "/root/test.iso".into(),
        target_path: "D:/test.iso".into(),
        direction: TransferDirection::Download,
        total_bytes: 50_000_000,
        transferred_bytes: 50_000_000,
        speed_bytes_per_sec: 25_000_000,
        status: TransferStatus::Completed,
        error_message: None,
    };

    let _ = engine.dispatch_file_transfer_enqueued(&task);
    engine.dispatch_file_transfer_started("task-002");
    engine.dispatch_file_transfer_progress("task-002", 25_000_000, 50_000_000, 25_000_000);
    engine.dispatch_file_transfer_completed(&task);
    engine.dispatch_file_transfer_failed(&task, "连接超时");
}



