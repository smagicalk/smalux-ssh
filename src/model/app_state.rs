//! Iced 应用根状态和消息调度。

use iced::Task;
use iced::Theme;

use std::fmt;

use crate::backend::{BackendCommandQueue, BackendEvent, apply_backend_event};
use crate::backend::{
    SharedBackendExecutor, noop_shared_backend_executor, shared_backend_executor,
};
use crate::config::AppConfig;
use crate::session::SessionManager;
use crate::storage::StorageManager;
use crate::terminal::TerminalManager;

use super::{HostId, TunnelRule};

mod backend_pump;
#[cfg(test)]
mod backend_pump_tests;
mod launch;
#[cfg(test)]
mod launch_tests;
#[cfg(test)]
mod tests;

/// Iced 应用的根状态。
///
/// 根状态只组合各个单一职责管理器，不直接实现 SSH、SFTP 或终端细节。
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionManager,
    pub storage: StorageManager,
    pub terminal: TerminalManager,
    pub backend_commands: BackendCommandQueue,
    pub backend_executor: SharedBackendExecutor,
    pub theme: Theme,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .field("sessions", &self.sessions)
            .field("storage", &self.storage)
            .field("terminal", &self.terminal)
            .field("backend_commands", &self.backend_commands)
            .field("backend_executor", &"<shared backend executor>")
            .field("theme", &self.theme)
            .finish()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            sessions: SessionManager::default(),
            storage: StorageManager::default(),
            terminal: TerminalManager::default(),
            backend_commands: BackendCommandQueue::default(),
            backend_executor: noop_shared_backend_executor(),
            theme: Theme::Dark,
        }
    }
}

/// UI 与后台任务之间传递的消息。
#[derive(Debug, Clone)]
pub enum Message {
    ToggleTheme,
    OpenShell {
        host_id: HostId,
    },
    OpenSftp {
        host_id: HostId,
        initial_dir: String,
    },
    RunRemoteCommand {
        host_id: HostId,
        command: String,
        request_pty: bool,
    },
    StartTunnel {
        host_id: HostId,
        rule: TunnelRule,
    },
    StopTunnel {
        session_id: crate::model::SessionId,
        rule_name: String,
    },
    BackendEventReceived(BackendEvent),
}

/// 应用消息处理结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppUpdateOutcome {
    pub state_changed: bool,
    pub queued_backend_commands: usize,
    pub executed_backend_commands: usize,
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
    /// 使用指定共享执行器替换默认占位执行器。
    pub fn with_backend_executor<E>(mut self, executor: E) -> Self
    where
        E: crate::backend::BackendExecutor + 'static,
    {
        self.backend_executor = shared_backend_executor(executor);
        self
    }

    /// 构造 Iced 启动需要的初始状态和首个任务。
    pub fn boot() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    /// 将 UI 消息应用到根状态。
    pub fn apply(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::ToggleTheme => self.toggle_theme(),
            Message::OpenShell { host_id } => self.open_shell(host_id),
            Message::OpenSftp {
                host_id,
                initial_dir,
            } => self.open_sftp(host_id, initial_dir),
            Message::RunRemoteCommand {
                host_id,
                command,
                request_pty,
            } => self.run_remote_command(host_id, command, request_pty),
            Message::StartTunnel { host_id, rule } => self.start_tunnel(host_id, rule),
            Message::StopTunnel {
                session_id,
                rule_name,
            } => self.stop_tunnel(session_id, rule_name),
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

    /// 使用当前共享执行器泵出已排队的后台命令。
    pub fn drain_backend_queue_with_executor(&mut self) -> AppUpdateOutcome {
        let backend_executor = self.backend_executor.clone();
        let mut executor = backend_executor
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        self.drain_backend_queue(&mut **executor)
    }
}
