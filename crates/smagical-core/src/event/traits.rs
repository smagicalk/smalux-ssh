//! 事件标记 Trait (AppEvent)。
//!
//! 所有可以通过 `EventDispatcher` 广播与订阅的强类型事件均需实现此 Trait。

use std::any::Any;

/// 全局事件标记 Trait。
///
/// 任何满足 `'static + Send + Sync` 的类型均可自动获得此 Trait 的实现。
pub trait AppEvent: Send + Sync + 'static {
    /// 获取事件可读类型名称 (默认使用编译器生成的类型路径，用于日志追踪与调试可观测)。
    fn event_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// 支持动态转换为 Any 引用，以便在泛型分发器内部安全向下类型擦除与还原。
    fn as_any(&self) -> &dyn Any;
}

impl<T: Send + Sync + 'static> AppEvent for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
