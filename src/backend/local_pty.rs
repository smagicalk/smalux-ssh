pub use smagical_local_backend::{DesktopBackendExecutor, LocalPtyBackendExecutor};

/// 当前默认运行时使用的组合执行器。
///
/// 它把本地 PTY fallback 和远程 SSH/SFTP/隧道执行器组合起来，但不依赖任何具体 UI。
pub type RuntimeBackendExecutor<R> = DesktopBackendExecutor<R>;

pub fn default_runtime_backend_executor<R>(remote: R) -> RuntimeBackendExecutor<R> {
    DesktopBackendExecutor::new(remote)
}
