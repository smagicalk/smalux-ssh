//! 强类型事件驱动通信与多实例分发器系统。
//!
//! 提供基于 `TypeId` 哈希索引的通用事件分发器 (`EventDispatcher`)、
//! 自动注销守卫 (`ListenerGuard`)、集中管理器 (`EventManager`) 与核心领域事件定义。

pub mod dispatcher;
pub mod guard;
pub mod manager;
pub mod traits;
pub mod types;

#[cfg(test)]
mod tests;

pub use dispatcher::EventDispatcher;
pub use guard::ListenerGuard;
pub use manager::EventManager;
pub use traits::AppEvent;
pub use types::*;
