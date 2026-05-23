//! 应用根状态和消息调度。

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
mod snippets;
#[cfg(test)]
mod snippets_tests;
mod state;
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

pub use message::Message;
pub use outcome::AppUpdateOutcome;
pub use state::AppState;
