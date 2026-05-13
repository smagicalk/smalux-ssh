//! 后端任务接口层。
//!
//! 本模块只定义 UI 状态层与真实 SSH/PTY/SFTP/隧道执行器之间交换的命令和事件。
//! 具体 `russh`、`russh-sftp`、`portable-pty` 句柄会放在后续执行器模块，避免进入可克隆的 UI 状态。

mod auth;
mod command;
mod event;
mod executor;
mod pty;
mod queue;
mod reducer;
mod sftp;
mod tunnel;

pub use auth::*;
pub use command::*;
pub use event::*;
pub use executor::*;
pub use pty::*;
pub use queue::*;
pub use reducer::*;
pub use sftp::*;
pub use tunnel::*;
