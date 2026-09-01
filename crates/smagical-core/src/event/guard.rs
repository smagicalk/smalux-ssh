//! RAII 自动注销守卫 (ListenerGuard)。
//!
//! 当监听者通过 `EventDispatcher::listen` 订阅事件时，将返回此守卫实例。
//! 一旦守卫离开作用域被析构 (Drop)，将自动触发注销逻辑，杜绝内存泄漏与悬垂闭包。

/// RAII 自动注销守卫。
pub struct ListenerGuard {
    id: u64,
    unreg_fn: Option<Box<dyn FnOnce(u64) + Send + Sync>>,
}

impl ListenerGuard {
    /// 创建一个新的注销守卫。
    pub fn new<F>(id: u64, unregister: F) -> Self
    where
        F: FnOnce(u64) + Send + Sync + 'static,
    {
        Self {
            id,
            unreg_fn: Some(Box::new(unregister)),
        }
    }

    /// 获取当前监听者的唯一标识 ID。
    pub fn id(&self) -> u64 {
        self.id
    }

    /// 显式脱离守卫生命周期。
    ///
    /// 调用此方法后，监听者将永久常驻，即使守卫被 drop 也不会注销。
    pub fn detach(mut self) {
        self.unreg_fn = None;
    }
}

impl Drop for ListenerGuard {
    fn drop(&mut self) {
        if let Some(unreg) = self.unreg_fn.take() {
            unreg(self.id);
        }
    }
}
