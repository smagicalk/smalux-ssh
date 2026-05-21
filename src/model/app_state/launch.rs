//! Shell 会话启动和共享的后端命令调度辅助。

#[path = "launch/host.rs"]
mod host;
#[path = "launch/outcome.rs"]
mod outcome;
#[path = "launch/path.rs"]
mod path;
#[path = "launch/shell.rs"]
mod shell;
#[path = "launch/time.rs"]
mod time;

pub(in crate::model::app_state) use host::connect_command_with_known_hosts;
pub(in crate::model::app_state) use outcome::{missing_host, queued_outcome};
pub(in crate::model::app_state) use path::{join_remote_path, normalize_remote_dir};
pub(in crate::model::app_state) use time::unix_now_secs;
