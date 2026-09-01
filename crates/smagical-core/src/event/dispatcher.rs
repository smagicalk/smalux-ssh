//! 强类型通用事件分发器 (EventDispatcher)。
//!
//! 具备 O(1) TypeId 哈希路由索引、并发读写分离、临界区最小化防死锁、RAII 自动注销与 Panic 沙箱隔离。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock, Weak};

use super::guard::ListenerGuard;
use super::traits::AppEvent;

type EventHandler = Box<dyn Fn(&dyn Any) + Send + Sync>;

struct ListenerEntry {
    id: u64,
    handler: EventHandler,
}

/// 通用强类型事件分发器。
///
/// 支持并发安全的多生产者多分发，内部基于 `TypeId` 哈希表实现零开销事件路由。
pub struct EventDispatcher {
    next_id: AtomicU64,
    listeners: Arc<RwLock<HashMap<TypeId, Vec<Arc<ListenerEntry>>>>>,
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDispatcher {
    /// 创建一个新的事件分发器实例。
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 订阅特定类型的强类型事件。
    ///
    /// # 返回
    /// 返回 `ListenerGuard`，当 Guard 离开作用域被析构时，将自动从分发器中注销该监听者。
    pub fn listen<E: AppEvent, F>(&self, handler: F) -> ListenerGuard
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let type_id = TypeId::of::<E>();

        let wrapped: EventHandler = Box::new(move |any_event| {
            if let Some(e) = any_event.downcast_ref::<E>() {
                handler(e);
            }
        });

        let entry = Arc::new(ListenerEntry { id, handler: wrapped });

        {
            let mut map = self.listeners.write().unwrap();
            map.entry(type_id).or_default().push(entry);
        }

        let map_weak: Weak<RwLock<HashMap<TypeId, Vec<Arc<ListenerEntry>>>>> = Arc::downgrade(&self.listeners);

        ListenerGuard::new(id, move |unreg_id| {
            if let Some(map_arc) = map_weak.upgrade() {
                if let Ok(mut map) = map_arc.write() {
                    if let Some(entries) = map.get_mut(&type_id) {
                        entries.retain(|e| e.id != unreg_id);
                    }
                }
            }
        })
    }

    /// 广播特定类型的强类型事件。
    ///
    /// # 防死锁与安全性保障
    /// 1. 仅在短暂的读锁内复制监听者引用列表，随后立即释放锁，保证回调执行期间无锁阻塞；
    /// 2. 每个回调均置于 `panic::catch_unwind` 沙箱中执行，单个监听者异常不影响后续分发。
    pub fn dispatch<E: AppEvent>(&self, event: &E) {
        let type_id = TypeId::of::<E>();

        // 1. 【临界区极小化】快速提取当前事件的监听者列表引用，立即释放锁
        let handlers_to_call: Vec<Arc<ListenerEntry>> = {
            let map = self.listeners.read().unwrap();
            map.get(&type_id).cloned().unwrap_or_default()
        };

        // 2. 【无锁状态下并发安全执行】
        for entry in handlers_to_call {
            let handler_cloned = Arc::clone(&entry);
            let any_event = event.as_any();
            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                (handler_cloned.handler)(any_event);
            }));
        }
    }

    /// 获取当前分发器内已注册的事件类型数与监听者总条数。
    pub fn stats(&self) -> (usize, usize) {
        let map = self.listeners.read().unwrap();
        let event_types = map.len();
        let total_listeners = map.values().map(|v| v.len()).sum();
        (event_types, total_listeners)
    }

    /// 清空分发器中所有已注册的监听者。
    pub fn clear(&self) {
        let mut map = self.listeners.write().unwrap();
        map.clear();
    }
}
