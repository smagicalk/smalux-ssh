//! 事件分发器与管理器单元测试。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use super::dispatcher::EventDispatcher;
use super::manager::EventManager;
use super::types::{CredentialSavedEvent, KeyGeneratedEvent};
use crate::domain::credential::CredentialType;

#[test]
fn test_event_dispatcher_basic_pub_sub() {
    let dispatcher = EventDispatcher::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let counter_clone = Arc::clone(&counter);
    let _guard = dispatcher.listen::<CredentialSavedEvent, _>(move |event| {
        assert_eq!(event.cred_id, "cred-001");
        assert_eq!(event.name, "生产密钥");
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(dispatcher.stats(), (1, 1));

    // 触发分发
    dispatcher.dispatch(&CredentialSavedEvent {
        cred_id: "cred-001".into(),
        name: "生产密钥".into(),
        cred_type: CredentialType::Key,
        algorithm: "Ed25519".into(),
        username: Some("root".into()),
        fingerprint: Some("SHA256:abc".into()),
        is_new: true,
    });

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_raii_guard_auto_unregister() {
    let dispatcher = EventDispatcher::new();
    let called = Arc::new(AtomicBool::new(false));

    // 1. 注册并在作用域结束时 drop guard
    {
        let called_clone = Arc::clone(&called);
        let _guard = dispatcher.listen::<KeyGeneratedEvent, _>(move |_| {
            called_clone.store(true, Ordering::SeqCst);
        });
        assert_eq!(dispatcher.stats(), (1, 1));
    }

    // 此时 guard 已 drop，监听者已被自动反注册
    assert_eq!(dispatcher.stats(), (1, 0));

    dispatcher.dispatch(&KeyGeneratedEvent {
        algorithm: "Ed25519".into(),
        fingerprint: "SHA256:xyz".into(),
    });

    // 确认未被调用
    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn test_guard_detach_stays_alive() {
    let dispatcher = EventDispatcher::new();
    let count = Arc::new(AtomicUsize::new(0));

    {
        let count_clone = Arc::clone(&count);
        let guard = dispatcher.listen::<KeyGeneratedEvent, _>(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });
        guard.detach(); // 显式脱离生命周期
    }

    assert_eq!(dispatcher.stats(), (1, 1));

    dispatcher.dispatch(&KeyGeneratedEvent {
        algorithm: "RSA-4096".into(),
        fingerprint: "SHA256:rsa".into(),
    });

    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_multi_dispatcher_explicit_forwarding() {
    let comp_bus = EventDispatcher::new();
    let page_bus = Arc::new(EventDispatcher::new());
    let global_bus = Arc::new(EventDispatcher::new());

    let global_received = Arc::new(AtomicBool::new(false));
    let global_rec_clone = Arc::clone(&global_received);

    // 全局总线监听 CredentialSavedEvent
    let _g_guard = global_bus.listen::<CredentialSavedEvent, _>(move |e| {
        assert_eq!(e.name, "由组件生成的密钥");
        global_rec_clone.store(true, Ordering::SeqCst);
    });

    // 页面总线监听 KeyGeneratedEvent，并【显式手动转发】给全局总线
    let g_bus_for_page = Arc::clone(&global_bus);
    let _p_guard = page_bus.listen::<KeyGeneratedEvent, _>(move |keygen| {
        // 页面执行本地持久化后，显式构造全局事件并向全局总线广播
        g_bus_for_page.dispatch(&CredentialSavedEvent {
            cred_id: "cred-gen".into(),
            name: "由组件生成的密钥".into(),
            cred_type: CredentialType::Key,
            algorithm: keygen.algorithm.clone(),
            username: None,
            fingerprint: Some(keygen.fingerprint.clone()),
            is_new: true,
        });
    });

    // 组件总线触发局部事件，并显式转发给页面总线
    let p_bus_for_comp = Arc::clone(&page_bus);
    let _c_guard = comp_bus.listen::<KeyGeneratedEvent, _>(move |keygen| {
        p_bus_for_comp.dispatch(keygen);
    });

    // 用户在组件中点击生成
    comp_bus.dispatch(&KeyGeneratedEvent {
        algorithm: "Ed25519".into(),
        fingerprint: "SHA256:ed25519_test".into(),
    });

    // 验证链路：组件 ➔ 页面 ➔ 全局 成功触达！
    assert!(global_received.load(Ordering::SeqCst));
}

#[test]
fn test_panic_safety_isolation() {
    let dispatcher = EventDispatcher::new();
    let normal_called = Arc::new(AtomicBool::new(false));

    // 注册一个会 panic 的监听者
    let _panic_guard = dispatcher.listen::<KeyGeneratedEvent, _>(|_| {
        panic!("测试故意抛出的 panic");
    });

    // 注册一个正常监听者
    let normal_clone = Arc::clone(&normal_called);
    let _normal_guard = dispatcher.listen::<KeyGeneratedEvent, _>(move |_| {
        normal_clone.store(true, Ordering::SeqCst);
    });

    // 分发事件：panic 应当被沙箱捕获，正常监听者必须正常执行
    dispatcher.dispatch(&KeyGeneratedEvent {
        algorithm: "Ed25519".into(),
        fingerprint: "SHA256:safe".into(),
    });

    assert!(normal_called.load(Ordering::SeqCst));
}

#[test]
fn test_event_manager_lifecycle() {
    let mgr = EventManager::new();
    assert_eq!(mgr.active_page_ids().len(), 0);
    assert_eq!(mgr.active_component_ids().len(), 0);

    // 获取/创建页面分发器
    let page1 = mgr.get_or_create_page("page.credentials");
    let _p1_guard = page1.listen::<CredentialSavedEvent, _>(|_| {});

    let comp1 = mgr.get_or_create_component("comp.keygen_modal");
    let _c1_guard = comp1.listen::<KeyGeneratedEvent, _>(|_| {});

    assert_eq!(mgr.active_page_ids(), vec!["page.credentials".to_string()]);
    assert_eq!(mgr.active_component_ids(), vec!["comp.keygen_modal".to_string()]);
    assert_eq!(mgr.total_listener_count(), 2);

    // 移除组件分发器
    mgr.remove_component("comp.keygen_modal");
    assert_eq!(mgr.active_component_ids().len(), 0);
    assert_eq!(mgr.total_listener_count(), 1);

    // 移除页面分发器
    mgr.remove_page("page.credentials");
    assert_eq!(mgr.active_page_ids().len(), 0);
    assert_eq!(mgr.total_listener_count(), 0);
}

#[test]
fn test_tunnel_lifecycle_events_and_abortion() {
    let dispatcher = EventDispatcher::new();

    // 1. 测试保存前拦截 (如端口或跳板成环)
    let _g_save = dispatcher.listen::<crate::event::TunnelBeforeSaveEvent, _>(|e| {
        if e.local_port == 0 {
            e.abort("端口不能为 0");
        }
    });

    let ev_invalid = crate::event::TunnelBeforeSaveEvent::new("tun-1", "测试", "Local", "127.0.0.1", 0, "10.0.0.1", 80, vec![]);
    dispatcher.dispatch(&ev_invalid);
    assert!(ev_invalid.is_aborted());
    assert_eq!(ev_invalid.abort_reason(), Some("端口不能为 0".into()));

    let ev_valid = crate::event::TunnelBeforeSaveEvent::new("tun-2", "正常", "Local", "127.0.0.1", 8080, "10.0.0.1", 80, vec![]);
    dispatcher.dispatch(&ev_valid);
    assert!(!ev_valid.is_aborted());

    // 2. 测试运行中删除拦截
    let _g_del = dispatcher.listen::<crate::event::TunnelBeforeDeleteEvent, _>(|e| {
        if e.is_running {
            e.abort("运行中禁止删除");
        }
    });

    let del_running = crate::event::TunnelBeforeDeleteEvent::new("tun-1", true);
    dispatcher.dispatch(&del_running);
    assert!(del_running.is_aborted());
    assert_eq!(del_running.abort_reason(), Some("运行中禁止删除".into()));

    let del_stopped = crate::event::TunnelBeforeDeleteEvent::new("tun-1", false);
    dispatcher.dispatch(&del_stopped);
    assert!(!del_stopped.is_aborted());

    // 3. 测试流量度量事件
    let metrics_seen = Arc::new(AtomicUsize::new(0));
    let m_clone = Arc::clone(&metrics_seen);
    let _g_metric = dispatcher.listen::<crate::event::TunnelMetricsTickEvent, _>(move |e| {
        m_clone.fetch_add(e.bytes_in_delta as usize, Ordering::SeqCst);
    });

    dispatcher.dispatch(&crate::event::TunnelMetricsTickEvent {
        tunnel_id: "tun-1".into(),
        bytes_in_delta: 1024,
        bytes_out_delta: 2048,
        active_connections: 5,
    });
    assert_eq!(metrics_seen.load(Ordering::SeqCst), 1024);
}

