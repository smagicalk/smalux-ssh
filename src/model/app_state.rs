//! 核心消息与状态动作层。
//!
//! 这里定义应用操作语言 `Message` 以及按功能拆分的核心动作模块。桌面适配层会在
//! 此基础上组合 `CoreState + UiState`；未来替换 UI 时，优先复用这里。

#[cfg(test)]
use super::{HostId, SnippetId, VisualSettingsDraftField, WorkspacePage};

mod backend_events;
mod backend_pump;
#[cfg(test)]
mod backend_pump_tests;
mod credentials;
mod dispatch;
mod host_editor;
mod host_records;
mod known_hosts;
mod launch;
mod launch_remote_command;
mod launch_sftp;
mod launch_sftp_transfer;
#[cfg(test)]
mod launch_tests;
mod launch_tunnel;
mod message;
mod network_assets;
mod outcome;
mod session_tabs;
mod settings;
#[cfg(test)]
mod settings_tests;
mod snippets;
#[cfg(test)]
mod snippets_tests;
#[cfg(test)]
#[path = "app_state/storage_admin/tests.rs"]
mod storage_tests;
mod terminal_input;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_drafts_tests;
mod visual_settings;
#[cfg(test)]
mod visual_settings_tests;
mod workspace;
#[cfg(test)]
#[path = "app_state/workspace_ui/tests.rs"]
mod workspace_ui_tests;

pub use backend_pump::BackendCommandResult;
pub use message::Message;
pub use outcome::AppUpdateOutcome;
