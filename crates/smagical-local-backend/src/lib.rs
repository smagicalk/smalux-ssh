//! 本地桌面后端执行器。
//!
//! 只负责本地 shell 进程、PTY 读写和 fallback 命令执行，不读取 UI 状态，也不处理远程 SSH。

mod local_command;
mod local_pty;

pub use local_command::{LocalCommandFallback, LocalCommandFallbackResult};
pub use local_pty::{DesktopBackendExecutor, LocalPtyBackendExecutor};
