//! 领域模型聚合和应用根状态。
//!
//! 具体领域类型按单一职责拆分到 `src/model/` 子模块中，本文件只负责对外导出稳定 API，
//! 并保留 Iced 应用根状态，避免单文件继续膨胀。

mod history;
mod host;
mod ids;
mod security;
mod session;
mod sftp;
mod snippet;
mod tunnel;
mod visual;
mod workspace;

pub use history::*;
pub use host::*;
pub use ids::*;
pub use security::*;
pub use session::*;
pub use sftp::*;
pub use snippet::*;
pub use tunnel::*;
pub use visual::*;
pub use workspace::*;

use iced::Task;
use iced::Theme;

use crate::config::AppConfig;
use crate::session::SessionManager;
use crate::storage::StorageManager;
use crate::terminal::TerminalManager;

/// Iced 应用的根状态。
///
/// 根状态只组合各个单一职责管理器，不直接实现 SSH、SFTP 或终端细节。
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionManager,
    pub storage: StorageManager,
    pub terminal: TerminalManager,
    pub theme: Theme,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            sessions: SessionManager::default(),
            storage: StorageManager::default(),
            terminal: TerminalManager::default(),
            theme: Theme::Dark,
        }
    }
}

/// UI 与后台任务之间传递的消息。
#[derive(Debug, Clone)]
pub enum Message {
    ToggleTheme,
}

impl AppState {
    /// 构造 Iced 启动需要的初始状态和首个任务。
    pub fn boot() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    /// 将 UI 消息应用到根状态。
    pub fn apply(&mut self, message: Message) {
        match message {
            Message::ToggleTheme => {
                self.theme = if matches!(self.theme, Theme::Dark) {
                    Theme::Light
                } else {
                    Theme::Dark
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_starts_empty_and_dark() {
        let state = AppState::default();

        assert_eq!(state.config.app_name, "smagicalssh");
        assert_eq!(state.sessions.active_count(), 0);
        assert_eq!(state.storage.host_count(), 0);
        assert_eq!(state.terminal.tab_count(), 0);
        assert!(matches!(state.theme, Theme::Dark));
    }

    #[test]
    fn boot_returns_default_state_without_startup_task() {
        let (state, _task) = AppState::boot();

        assert_eq!(state.config.app_name, "smagicalssh");
        assert!(matches!(state.theme, Theme::Dark));
    }

    #[test]
    fn toggle_theme_switches_between_dark_and_light() {
        let mut state = AppState::default();

        state.apply(Message::ToggleTheme);
        assert!(matches!(state.theme, Theme::Light));

        state.apply(Message::ToggleTheme);
        assert!(matches!(state.theme, Theme::Dark));
    }

    #[test]
    fn root_module_reexports_domain_types() {
        let host_id = HostId(uuid::Uuid::new_v4());
        let tab = SessionTab {
            id: SessionId(uuid::Uuid::new_v4()),
            host_id: Some(host_id),
            kind: SessionKind::Shell,
            title: "shell".to_owned(),
            status: SessionStatus::Connecting,
        };

        assert_eq!(tab.host_id, Some(host_id));
    }
}
