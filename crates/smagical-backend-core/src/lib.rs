//! 后端任务接口层。
//!
//! 本模块只定义状态层与真实 SSH/PTY/SFTP/隧道执行器之间交换的命令、事件和纯请求模型。
//! 具体 `russh`、`russh-sftp`、`portable-pty` 句柄留在应用后端执行器中，避免进入可克隆的 UI 状态。

mod auth;
mod command;
mod event;
mod executor;
mod local_shell;
mod pty;
mod queue;
mod sftp;
mod tunnel;

pub use auth::*;
pub use command::*;
pub use event::*;
pub use executor::*;
pub use local_shell::*;
pub use pty::*;
pub use queue::*;
pub use sftp::*;
pub use tunnel::*;
