//! Iced 应用根状态和消息调度。

use iced::Task;
use iced::Theme;
use uuid::Uuid;

use crate::backend::{
    BackendCommand, BackendCommandQueue, BackendEvent, ConnectionTarget, PtyRequest,
    apply_backend_event,
};
use crate::config::AppConfig;
use crate::session::SessionManager;
use crate::storage::StorageManager;
use crate::terminal::{TerminalManager, TerminalTabState};

use super::{HostId, SessionId, SessionStatus};

#[cfg(test)]
mod tests;

/// Iced 应用的根状态。
///
/// 根状态只组合各个单一职责管理器，不直接实现 SSH、SFTP 或终端细节。
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionManager,
    pub storage: StorageManager,
    pub terminal: TerminalManager,
    pub backend_commands: BackendCommandQueue,
    pub theme: Theme,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            sessions: SessionManager::default(),
            storage: StorageManager::default(),
            terminal: TerminalManager::default(),
            backend_commands: BackendCommandQueue::default(),
            theme: Theme::Dark,
        }
    }
}

/// UI 与后台任务之间传递的消息。
#[derive(Debug, Clone)]
pub enum Message {
    ToggleTheme,
    OpenShell { host_id: HostId },
    BackendEventReceived(BackendEvent),
}

/// 应用消息处理结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppUpdateOutcome {
    pub state_changed: bool,
    pub queued_backend_commands: usize,
    pub applied_backend_events: usize,
    pub error: Option<String>,
}

impl AppUpdateOutcome {
    /// 是否有状态变化或错误反馈。
    pub fn changed(&self) -> bool {
        self.state_changed || self.error.is_some()
    }
}

impl AppState {
    /// 构造 Iced 启动需要的初始状态和首个任务。
    pub fn boot() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    /// 将 UI 消息应用到根状态。
    pub fn apply(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::ToggleTheme => self.toggle_theme(),
            Message::OpenShell { host_id } => self.open_shell(host_id),
            Message::BackendEventReceived(event) => self.apply_backend_event(event),
        }
    }

    fn toggle_theme(&mut self) -> AppUpdateOutcome {
        self.theme = if matches!(self.theme, Theme::Dark) {
            Theme::Light
        } else {
            Theme::Dark
        };

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    fn apply_backend_event(&mut self, event: BackendEvent) -> AppUpdateOutcome {
        let outcome = apply_backend_event(&mut self.sessions, &mut self.terminal, event);

        AppUpdateOutcome {
            state_changed: outcome.changed(),
            applied_backend_events: 1,
            ..AppUpdateOutcome::default()
        }
    }

    fn open_shell(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(host) = self
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到主机：{}", host_id.0)),
                ..AppUpdateOutcome::default()
            };
        };

        let session_id = SessionId(Uuid::new_v4());
        let terminal_tab = TerminalTabState::new(session_id, host.name.clone());
        let pty = PtyRequest::xterm(terminal_tab.size);

        self.sessions
            .open_shell_tab(session_id, host.id, host.name.clone());
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.terminal.open_tab(terminal_tab);
        self.backend_commands.extend([
            BackendCommand::Connect {
                session_id,
                target: ConnectionTarget::from_host(&host),
            },
            BackendCommand::OpenShell { session_id, pty },
        ]);

        AppUpdateOutcome {
            state_changed: true,
            queued_backend_commands: 2,
            ..AppUpdateOutcome::default()
        }
    }
}
