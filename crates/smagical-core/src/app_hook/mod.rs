//! 全局应用级生命周期、主框架导航与配置变更 Hook 体系。

/// 内置应用级 Hook 插件。
pub mod builtin;
/// 全局 Hook 调度引擎。
pub mod engine;
/// Hook 核心 Trait 接口定义。
pub mod traits;
/// 上下文与事件类型定义。
pub mod types;


#[cfg(test)]
mod tests;

pub use builtin::{AutoConfigBackupHook, FunctionalGlobalHook};
pub use engine::{AppGlobalHookEngine, ListenerHandle};
pub use traits::AppGlobalHook;
pub use types::{AppBootContext, AppExitContext, ConfigChangeEvent, WindowState};
