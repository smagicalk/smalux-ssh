//! 主机操作区输入草稿状态。

#[path = "host_action/types.rs"]
mod types;
#[path = "host_action/ui.rs"]
mod ui;

pub use types::{DEFAULT_REMOTE_COMMAND, DEFAULT_SFTP_INITIAL_DIR, HostActionDraft};
