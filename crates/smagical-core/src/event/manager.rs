//! 集中式事件分发管理器 (EventManager)。
//!
//! 负责集中索引和生命周期管理全局分发器 (Global)、页面分发器 (Page) 与组件分发器 (Component)。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::dispatcher::EventDispatcher;

/// 集中式事件分发与作用域管理器。
///
/// 统一管理全系统常驻的全局分发器，以及按需动态创建、查找与注销的页面级与组件级分发器。
pub struct EventManager {
    /// 全局唯一共享事件分发器 (生命周期与应用进程相同)。
    global: Arc<EventDispatcher>,
    /// 动态页面分发器哈希表 (由 Page ID 索引)。
    pages: RwLock<HashMap<String, Arc<EventDispatcher>>>,
    /// 动态组件分发器哈希表 (由 Component ID 索引)。
    components: RwLock<HashMap<String, Arc<EventDispatcher>>>,
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EventManager {
    /// 创建一个新的事件管理器实例。
    pub fn new() -> Self {
        Self {
            global: Arc::new(EventDispatcher::new()),
            pages: RwLock::new(HashMap::new()),
            components: RwLock::new(HashMap::new()),
        }
    }

    /// 获取全局事件分发器引用。
    pub fn global(&self) -> &Arc<EventDispatcher> {
        &self.global
    }

    /// 获取或惰性创建指定 Page ID 的页面事件分发器。
    pub fn get_or_create_page(&self, page_id: &str) -> Arc<EventDispatcher> {
        let mut map = self.pages.write().unwrap();
        map.entry(page_id.to_string())
            .or_insert_with(|| Arc::new(EventDispatcher::new()))
            .clone()
    }

    /// 移除并释放指定页面的事件分发器 (页面关闭时调用以回收资源)。
    pub fn remove_page(&self, page_id: &str) {
        let mut map = self.pages.write().unwrap();
        map.remove(page_id);
    }

    /// 获取或惰性创建指定 Component ID 的组件事件分发器。
    pub fn get_or_create_component(&self, comp_id: &str) -> Arc<EventDispatcher> {
        let mut map = self.components.write().unwrap();
        map.entry(comp_id.to_string())
            .or_insert_with(|| Arc::new(EventDispatcher::new()))
            .clone()
    }

    /// 移除并释放指定组件的事件分发器 (弹窗/抽屉销毁时调用以回收资源)。
    pub fn remove_component(&self, comp_id: &str) {
        let mut map = self.components.write().unwrap();
        map.remove(comp_id);
    }

    /// 获取当前所有活跃页面的 ID 列表。
    pub fn active_page_ids(&self) -> Vec<String> {
        let map = self.pages.read().unwrap();
        map.keys().cloned().collect()
    }

    /// 获取当前所有活跃组件的 ID 列表。
    pub fn active_component_ids(&self) -> Vec<String> {
        let map = self.components.read().unwrap();
        map.keys().cloned().collect()
    }

    /// 汇总统计当前全局、页面与组件分发器内部的监听者总数。
    pub fn total_listener_count(&self) -> usize {
        let mut total = self.global.stats().1;
        {
            let p_map = self.pages.read().unwrap();
            for d in p_map.values() {
                total += d.stats().1;
            }
        }
        {
            let c_map = self.components.read().unwrap();
            for d in c_map.values() {
                total += d.stats().1;
            }
        }
        total
    }
}
