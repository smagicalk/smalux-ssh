//! 全局应用级生命周期、主框架导航与配置变更 Hook 体系。

pub mod builtin;
pub mod engine;
pub mod traits;
pub mod types;

#[cfg(test)]
mod tests;

pub use builtin::{AutoConfigBackupHook, FunctionalGlobalHook};
pub use engine::{AppGlobalHookEngine, ListenerHandle};
pub use traits::AppGlobalHook;
pub use types::{AppBootContext, AppExitContext, ConfigChangeEvent, WindowState};
