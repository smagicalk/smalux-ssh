//! 应用根状态和消息调度。
//!
//! 这个模块是桌面 UI 草稿和核心行为之间的过渡分发器：
//!
//! - `Message` 描述“用户或后台发生了什么”。
//! - `AppState::apply` 负责把仍依赖 UI 草稿的消息分发到对应单一职责模块。
//! - `AppUpdateOutcome` 描述一次状态变更是否产生了错误、后端命令或需要刷新。
//!
//! 新 UI 不需要知道 Slint 回调、Slint 模型或窗口属性；核心运行优先从
//! `CoreState` 开始，需要复用现有桌面交互草稿时再临时走 `AppState`。

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

pub use backend_pump::BackendCommandResult;
pub use message::Message;
pub use outcome::AppUpdateOutcome;
pub use state::AppState;
