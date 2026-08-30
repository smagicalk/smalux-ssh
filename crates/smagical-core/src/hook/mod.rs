//! 终端全生命周期 Hook、异常容错与输入输出时序追踪系统。

pub mod builtin;
pub mod decision;
pub mod engine;
pub mod error;
pub mod traits;
pub mod types;

#[cfg(test)]
mod tests;

pub use builtin::{DangerousCommandGuard, FunctionalHook, SessionAuditLogger};
pub use decision::{FallbackStrategy, HookDecision};
pub use engine::HookEngine;
pub use error::TerminalError;
pub use traits::TerminalHook;
pub use types::{
    CommandInteractionFrame, CommandSource, FrameStatus, HostMetadata, SessionContext,
};
