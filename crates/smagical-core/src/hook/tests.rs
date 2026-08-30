//! 终端 Hook 引擎单元测试。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::builtin::{DangerousCommandGuard, FunctionalHook, SessionAuditLogger};
use super::decision::HookDecision;
use super::engine::HookEngine;
use super::traits::TerminalHook;
use super::types::{CommandInteractionFrame, CommandSource, HostMetadata, SessionContext};


struct MockPriorityHook {
    name: &'static str,
    priority: i32,
    order_collector: Arc<std::sync::Mutex<Vec<&'static str>>>,
}

impl TerminalHook for MockPriorityHook {
    fn name(&self) -> &'static str {
        self.name
    }
    fn priority(&self) -> i32 {
        self.priority
    }
    fn on_command_completed(&self, _frame: &CommandInteractionFrame) {
        self.order_collector.lock().unwrap().push(self.name);
    }
}

struct PanicHook;
impl TerminalHook for PanicHook {
    fn name(&self) -> &'static str {
        "panic_hook"
    }
    fn on_command_completed(&self, _frame: &CommandInteractionFrame) {
        panic!("Mock unexpected plugin crash!");
    }
}

struct ModifyInputHook;
impl TerminalHook for ModifyInputHook {
    fn name(&self) -> &'static str {
        "modify_input_hook"
    }
    fn on_command_start(&self, frame: &CommandInteractionFrame) -> HookDecision<Vec<u8>> {
        if frame.command_line == "ll" {
            HookDecision::Modify(b"ls -la\n".to_vec())
        } else {
            HookDecision::Continue
        }
    }
}

#[test]
fn test_hook_registration_and_priority_sorting() {
    let engine = HookEngine::new();
    let collector = Arc::new(std::sync::Mutex::new(Vec::new()));

    engine.register(Arc::new(MockPriorityHook {
        name: "low_priority",
        priority: 0,
        order_collector: Arc::clone(&collector),
    }));

    engine.register(Arc::new(MockPriorityHook {
        name: "high_priority",
        priority: 100,
        order_collector: Arc::clone(&collector),
    }));

    engine.register(Arc::new(MockPriorityHook {
        name: "mid_priority",
        priority: 50,
        order_collector: Arc::clone(&collector),
    }));

    assert_eq!(engine.len(), 3);
    assert_eq!(
        engine.list_hooks(),
        vec!["high_priority", "mid_priority", "low_priority"]
    );

    let host = HostMetadata::local_shell("test-term");
    let session = SessionContext::new("sess-1", "pane-1", host);
    let mut frame = CommandInteractionFrame::new(1, session, "echo 1", CommandSource::Keyboard);
    frame.mark_completed(Some(0));

    engine.dispatch_command_completed(&frame);

    let executed_order = collector.lock().unwrap().clone();
    assert_eq!(
        executed_order,
        vec!["high_priority", "mid_priority", "low_priority"]
    );
}

#[test]
fn test_dangerous_command_guard_interception() {
    let engine = HookEngine::new();
    engine.register(Arc::new(DangerousCommandGuard::new()));

    let host = HostMetadata::remote_ssh("h1", "生产数据库", "10.0.0.1", 22, "root");
    let session = SessionContext::new("sess-prod", "pane-1", host);

    // 1. 普通命令允许通过
    let frame_safe = CommandInteractionFrame::new(1, session.clone(), "ls -la", CommandSource::Keyboard);
    let decision = engine.dispatch_command_start(&frame_safe);
    assert!(decision.is_continue());

    // 2. 高危命令拦截阻断
    let frame_danger = CommandInteractionFrame::new(2, session, "rm -rf / --no-preserve-root", CommandSource::Keyboard);
    let decision_danger = engine.dispatch_command_start(&frame_danger);
    assert!(decision_danger.is_aborted());
    if let HookDecision::Abort { reason } = decision_danger {
        assert!(reason.contains("高危破坏性指令"));
    } else {
        panic!("Expected HookDecision::Abort");
    }
}

#[test]
fn test_modify_input_pipeline() {
    let engine = HookEngine::new();
    engine.register(Arc::new(ModifyInputHook));

    let host = HostMetadata::local_shell("local");
    let session = SessionContext::new("sess-local", "pane-1", host);

    let frame = CommandInteractionFrame::new(1, session, "ll", CommandSource::Keyboard);
    let decision = engine.dispatch_command_start(&frame);
    assert_eq!(decision, HookDecision::Modify(b"ls -la\n".to_vec()));
}

#[test]
fn test_panic_safety_isolation() {
    let engine = HookEngine::new();
    let safe_executed = Arc::new(AtomicBool::new(false));
    let safe_flag = Arc::clone(&safe_executed);

    // 注册一个会崩溃的插件和一个正常的插件
    engine.register(Arc::new(PanicHook));
    engine.register(Arc::new(FunctionalHook::new(
        "safe_observer",
        -10,
        move |_frame| {
            safe_flag.store(true, Ordering::SeqCst);
        },
    )));

    let host = HostMetadata::local_shell("local");
    let session = SessionContext::new("sess-test", "pane-1", host);
    let frame = CommandInteractionFrame::new(1, session, "date", CommandSource::Keyboard);

    // 调度不应 panic，即使 PanicHook 抛出崩溃
    engine.dispatch_command_completed(&frame);

    // 安全插件依然成功执行
    assert!(safe_executed.load(Ordering::SeqCst));
}

#[test]
fn test_output_chunks_and_metrics_accumulation() {
    let host = HostMetadata::remote_ssh("h1", "测试机", "192.168.1.5", 22, "dev");
    let session = SessionContext::new("sess-out", "pane-1", host);
    let mut frame = CommandInteractionFrame::new(42, session, "cat log.txt", CommandSource::Keyboard);

    assert_eq!(frame.trace_id, "trace-sess-out-seq00042");
    assert!(frame.ttfb.is_none());

    // 模拟接收数据块
    frame.append_output(b"line 1\nline 2\n");
    assert!(frame.ttfb.is_some());
    assert_eq!(frame.output_lines, 2);
    assert_eq!(frame.output_text, "line 1\nline 2\n");

    frame.append_output(b"line 3\n");
    assert_eq!(frame.output_lines, 3);
    assert_eq!(frame.output_text, "line 1\nline 2\nline 3\n");

    frame.mark_completed(Some(0));
    assert_eq!(frame.exit_code, Some(0));
    assert_eq!(frame.status, super::types::FrameStatus::Completed);
}

#[test]
fn test_dynamic_unregister() {
    let engine = HookEngine::new();
    engine.register(Arc::new(SessionAuditLogger::new()));
    assert_eq!(engine.len(), 1);

    engine.unregister("session_audit_logger");
    assert_eq!(engine.len(), 0);
    assert!(engine.is_empty());
}

#[test]
fn test_history_tracking_hook_lifecycle() {
    let storage: Arc<dyn crate::storage::AppStorage> = Arc::new(crate::storage::MockStorage::new());
    let engine = HookEngine::new();

    engine.register(Arc::new(super::builtin::HistoryTrackingHook::new(Arc::clone(&storage))));

    let host = HostMetadata::remote_ssh("h-101", "生产Web服务", "10.0.0.8", 22, "deploy");
    let session = SessionContext::new("sess-h101", "pane-1", host);

    // 1. 会话开启 -> 自动写入活跃状态的历史记录
    engine.dispatch_post_open(&session);

    let hist = storage.history().get_by_id("hist-sess-h101").unwrap().expect("应已写入历史记录");
    assert_eq!(hist.title, "生产Web服务");
    assert_eq!(hist.address, "10.0.0.8:22");
    assert_eq!(hist.exit_status, "active");
    assert!(hist.disconnected_at.is_none());

    // 2. 会话关闭 -> 自动更新历史记录为 closed / success
    engine.dispatch_post_close(&session);

    let closed_hist = storage.history().get_by_id("hist-sess-h101").unwrap().expect("应找到历史记录");
    assert_eq!(closed_hist.exit_status, "success");
    assert!(closed_hist.disconnected_at.is_some());
}

