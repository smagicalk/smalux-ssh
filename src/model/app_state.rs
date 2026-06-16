//! 过渡期应用消息分发与历史兼容模块。
//!
//! 当前桌面主路径已经迁移到 `CoreState` 与 `DesktopAppState`。这里剩余模块主要用于
//! 清理最后一层历史兼容实现。

#[cfg(test)]
use super::{HostId, SnippetId, VisualSettingsDraftField, WorkspacePage};

mod backend_events;
mod backend_pump;
#[cfg(test)]
mod backend_pump_tests;
mod dispatch;
mod launch;
mod launch_remote_command;
mod launch_sftp;
mod launch_sftp_transfer;
#[cfg(test)]
mod launch_tests;
mod launch_tunnel;
mod message;
mod outcome;
mod session_tabs;
mod settings;
#[cfg(test)]
mod settings_tests;
mod snippets;
#[cfg(test)]
mod snippets_tests;
mod storage_admin;
#[cfg(test)]
mod tests;
mod ui_drafts;
#[cfg(test)]
mod ui_drafts_tests;
mod visual_settings;
#[cfg(test)]
mod visual_settings_tests;
mod workspace;
mod workspace_ui;

pub use backend_pump::BackendCommandResult;
pub use message::Message;
pub use outcome::AppUpdateOutcome;
